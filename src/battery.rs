use crate::icons::Kind;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags, OptionalExtension, types::ValueRef};
use serde_json::Value;
use std::{collections::BTreeMap, error::Error, path::Path, time::Duration};

pub type Devices = BTreeMap<Kind, Reading>;
pub type ReadError = Box<dyn Error>;

#[derive(Default)]
pub struct Alerts(std::collections::BTreeSet<Kind>);

impl Alerts {
    pub fn update(
        &mut self,
        devices: &Devices,
        thresholds: Thresholds,
        now: DateTime<Utc>,
    ) -> Vec<Kind> {
        let mut notifications = Vec::new();
        for (&kind, reading) in devices {
            if reading.percentage > thresholds.low {
                self.0.remove(&kind);
            }
            if reading.connected
                && !reading.stale(now)
                && reading.percentage <= thresholds.critical
                && self.0.insert(kind)
            {
                notifications.push(kind);
            }
        }
        notifications
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    pub name: String,
    pub percentage: u8,
    pub time: Option<DateTime<Utc>>,
    pub connected: bool,
}

impl Reading {
    pub fn stale(&self, now: DateTime<Utc>) -> bool {
        self.time
            .is_some_and(|time| (now - time).num_seconds() >= 3600)
    }

    pub fn label(&self, now: DateTime<Utc>) -> String {
        let mut text = format!("{}: {}%", self.name, self.percentage);
        if !self.connected {
            text.push_str(" (disconnected; cached)");
        }
        if let Some(time) = self.time.filter(|_| self.stale(now)) {
            let hours = (now - time).num_hours();
            text.push_str(&if hours >= 24 {
                format!(" (last seen {} d ago)", hours / 24)
            } else {
                format!(" (last seen {hours} h ago)")
            });
        }
        text
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Thresholds {
    pub critical: u8,
    pub low: u8,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            critical: 15,
            low: 30,
        }
    }
}

pub fn read_devices(path: &Path) -> Result<Devices, ReadError> {
    match path.metadata() {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Devices::new()),
        other => {
            other?;
        }
    }
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    connection.busy_timeout(Duration::from_millis(250))?;
    let json = connection
        .query_row(
            "SELECT file FROM data ORDER BY _id DESC LIMIT 1",
            [],
            |row| match row.get_ref(0)? {
                ValueRef::Text(bytes) | ValueRef::Blob(bytes) => Ok(bytes.to_vec()),
                value => Err(rusqlite::Error::InvalidColumnType(
                    0,
                    "file".into(),
                    value.data_type(),
                )),
            },
        )
        .optional()?;
    match json {
        Some(bytes) => parse(&serde_json::from_slice(&bytes)?),
        None => Ok(Devices::new()),
    }
}

pub fn parse(data: &Value) -> Result<Devices, ReadError> {
    let object = data
        .as_object()
        .ok_or("G HUB data.file must contain a JSON object")?;
    let known = data["devices/known"]["knownList"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut readings = Devices::new();
    for (key, value) in object {
        let parts: Vec<_> = key.split('/').collect();
        if parts.len() != 3 || parts[0] != "battery" || parts[2] != "percentage" {
            continue;
        }
        let Some(percentage) = value["percentage"].as_f64().filter(|n| n.is_finite()) else {
            continue;
        };
        let slug = parts[1].to_lowercase();
        let mut kind = Kind::classify(&slug);
        let mut best = 0.6;
        let mut model = None;
        let mut connected = value["connected"].as_bool().unwrap_or(true);
        for device in known {
            let Some(device_kind) = device["type"]
                .as_str()
                .and_then(|s| Kind::classify(&s.to_lowercase()))
            else {
                continue;
            };
            if kind.is_some_and(|kind| kind != device_kind) {
                continue;
            }
            let Some(id) = device["baseModelId"]
                .as_str()
                .filter(|s| !s.is_empty())
                .or_else(|| device["modelId"].as_str())
            else {
                continue;
            };
            let score = similarity(
                slug.as_bytes(),
                id.to_lowercase().replace('_', "").as_bytes(),
            );
            if score > best {
                best = score;
                model = Some((id, device_kind, device["connected"].as_bool()));
            }
        }
        if let Some((_, device_kind, status)) = model {
            kind = kind.or(Some(device_kind));
            connected &= status.unwrap_or(true);
        }
        let Some(kind) = kind else { continue };
        let name = model
            .map(|(id, _, _)| display_name(id))
            .unwrap_or_else(|| kind.name().into());
        let reading = Reading {
            name,
            percentage: percentage.clamp(0.0, 100.0) as u8,
            time: value["time"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|t| t.with_timezone(&Utc)),
            connected,
        };
        if readings
            .get(&kind)
            .is_none_or(|old| reading.time > old.time)
        {
            readings.insert(kind, reading);
        }
    }
    Ok(readings)
}

fn display_name(model: &str) -> String {
    model
        .split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            if word.len() <= 3 || word.chars().any(|c| c.is_ascii_digit()) {
                word.to_uppercase()
            } else {
                let mut chars = word.chars();
                format!(
                    "{}{}",
                    chars.next().unwrap().to_uppercase(),
                    chars.as_str().to_lowercase()
                )
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn similarity(a: &[u8], b: &[u8]) -> f64 {
    fn matches(a: &[u8], b: &[u8]) -> usize {
        let (mut ai, mut bi, mut length) = (0, 0, 0);
        for i in 0..a.len() {
            for j in 0..b.len() {
                let run = a[i..]
                    .iter()
                    .zip(&b[j..])
                    .take_while(|(x, y)| x == y)
                    .count();
                if run > length {
                    (ai, bi, length) = (i, j, run);
                }
            }
        }
        if length == 0 {
            return 0;
        }
        length + matches(&a[..ai], &b[..bi]) + matches(&a[ai + length..], &b[bi + length..])
    }
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    2.0 * matches(a, b) as f64 / (a.len() + b.len()) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_blocks_score_preserves_known_device_fallback() {
        assert_eq!(similarity(b"", b""), 1.0);
        assert_eq!(similarity(b"g915tkl", b"g915tkl"), 1.0);
        assert_eq!(similarity(b"mouse", b"headset"), 1.0 / 3.0);
        assert!(similarity(b"g502xlightspeed", b"g502xlightspeedwireless") > 0.6);
    }

    #[test]
    fn timestamp_ties_keep_first_reading_and_unknown_time_is_not_stale() {
        let readings = parse(&serde_json::json!({
            "battery/amouse/percentage": {"percentage": 50},
            "battery/bmouse/percentage": {"percentage": 90}
        }))
        .unwrap();
        assert_eq!(readings[&Kind::Mouse].percentage, 50);
        assert!(!readings[&Kind::Mouse].stale(Utc::now()));
    }
}
