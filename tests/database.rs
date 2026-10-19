use chrono::{TimeZone, Utc};
use logibar::{
    battery::{self, Alerts, Thresholds},
    icons::{self, Kind},
    options::Options,
};
use rusqlite::{Connection, params};
use serde_json::json;
use std::{ffi::OsString, fs};
use tempfile::TempDir;

fn fixture(blob: bool) -> (TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let connection = Connection::open(dir.path().join("settings.db")).unwrap();
    connection
        .execute_batch("CREATE TABLE data (_id INTEGER PRIMARY KEY, file BLOB NOT NULL);")
        .unwrap();
    connection
        .execute("INSERT INTO data VALUES (1, '{}')", [])
        .unwrap();
    let data = include_str!("fixtures/settings.json");
    if blob {
        connection
            .execute("INSERT INTO data VALUES (2, ?1)", params![data.as_bytes()])
            .unwrap();
    } else {
        connection
            .execute("INSERT INTO data VALUES (2, ?1)", params![data])
            .unwrap();
    }
    (dir, connection)
}

#[test]
fn actual_sqlite_latest_row_text_and_blob() {
    for blob in [false, true] {
        let (dir, _connection) = fixture(blob);
        let path = dir.path().join("settings.db");
        let before = fs::read(&path).unwrap();
        let devices = battery::read_devices(&path).unwrap();
        assert_eq!(devices.len(), 3);
        assert_eq!(
            devices[&Kind::Mouse].name,
            "PRO X 2 Superstrike Wireless Mouse"
        );
        assert_eq!(devices[&Kind::Mouse].percentage, 70);
        assert_eq!(devices[&Kind::Headset].percentage, 30);
        assert_eq!(devices[&Kind::Keyboard].percentage, 15);
        assert!(!devices[&Kind::Keyboard].connected);
        assert_eq!(before, fs::read(path).unwrap());
    }
}

#[test]
fn wal_committed_rows_visible_uncommitted_rows_ignored() {
    let (dir, connection) = fixture(false);
    connection
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;")
        .unwrap();
    let data = json!({"battery/livewirelessmouse/percentage": {"percentage": 42}}).to_string();
    connection
        .execute("INSERT INTO data VALUES (3, ?1)", params![data])
        .unwrap();
    let path = dir.path().join("settings.db");
    let before_db = fs::read(&path).unwrap();
    let before_wal = fs::read(dir.path().join("settings.db-wal")).unwrap();
    connection
        .execute_batch("BEGIN IMMEDIATE; INSERT INTO data VALUES (4, '{}');")
        .unwrap();
    assert_eq!(
        battery::read_devices(&path).unwrap()[&Kind::Mouse].percentage,
        42
    );
    assert_eq!(before_db, fs::read(&path).unwrap());
    assert_eq!(
        before_wal,
        fs::read(dir.path().join("settings.db-wal")).unwrap()
    );
    connection.execute_batch("ROLLBACK;").unwrap();
}

#[test]
fn missing_empty_corrupt_and_invalid_databases() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.db");
    assert!(battery::read_devices(&path).unwrap().is_empty());
    assert!(!path.exists());
    fs::write(&path, b"not SQLite").unwrap();
    assert!(battery::read_devices(&path).is_err());
    let (dir, connection) = fixture(false);
    let path = dir.path().join("settings.db");
    connection.execute("DELETE FROM data", []).unwrap();
    assert!(battery::read_devices(&path).unwrap().is_empty());
    for invalid in ["{", "[]", "null"] {
        connection
            .execute(
                "INSERT OR REPLACE INTO data VALUES (3, ?1)",
                params![invalid],
            )
            .unwrap();
        assert!(battery::read_devices(&path).is_err());
    }
}

#[test]
fn classification_fallback_names_and_malformed_values() {
    let data = json!({
        "devices/known": {"knownList": [null, {"type": "MOUSE", "modelId": "g502_x_lightspeed"}, {"type": "headphones", "modelId": "g733_headphone"}]},
        "battery/g502xlightspeed/percentage": {"percentage": 105, "time": "invalid"},
        "battery/g733headphone/percentage": {"percentage": -8},
        "battery/keyboard/percentage": {"percentage": true},
        "battery/camera/percentage": {"percentage": 50}
    });
    let devices = battery::parse(&data).unwrap();
    assert_eq!(devices.len(), 2);
    assert_eq!(devices[&Kind::Mouse].name, "G502 X Lightspeed");
    assert_eq!(devices[&Kind::Mouse].percentage, 100);
    assert_eq!(devices[&Kind::Mouse].time, None);
    assert_eq!(devices[&Kind::Headset].percentage, 0);
    assert!(
        battery::parse(&json!({"devices/known": false}))
            .unwrap()
            .is_empty()
    );
    assert!(
        battery::parse(&json!({"devices/known": {"knownList": "invalid"}}))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn timestamps_order_offsets_and_stale_disconnected_labels() {
    let data = json!({
        "battery/amouse/percentage": {"percentage": 10, "time": "2026-10-19T00:00:00-04:00"},
        "battery/bmouse/percentage": {"percentage": 90, "time": "2026-10-19T03:59:59Z"},
        "battery/headset/percentage": {"percentage": 20, "time": "2026-10-17T04:00:00Z", "connected": false}
    });
    let devices = battery::parse(&data).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 10, 19, 5, 0, 0).unwrap();
    assert_eq!(devices[&Kind::Mouse].percentage, 10);
    assert!(
        devices[&Kind::Mouse]
            .label(now)
            .contains("last seen 1 h ago")
    );
    assert!(
        devices[&Kind::Headset]
            .label(now)
            .contains("disconnected; cached")
    );
    assert!(
        devices[&Kind::Headset]
            .label(now)
            .contains("last seen 2 d ago")
    );
    assert!(!devices[&Kind::Mouse].stale(now - chrono::Duration::seconds(1)));
}

#[test]
fn thresholds_exact_boundaries_and_alert_hysteresis() {
    let normal = [255, 255, 255];
    for pct in [0, 15] {
        assert_eq!(icons::level_color(pct, 15, 30, normal), [255, 72, 72]);
    }
    for pct in [16, 30] {
        assert_eq!(icons::level_color(pct, 15, 30, normal), [255, 185, 0]);
    }
    for pct in [31, 100] {
        assert_eq!(icons::level_color(pct, 15, 30, normal), normal);
    }
    assert_eq!(icons::level_color(20, 20, 50, normal), [255, 72, 72]);
    let mut alerts = Alerts::default();
    let mut devices =
        battery::parse(&json!({"battery/mouse/percentage": {"percentage": 15}})).unwrap();
    let now = Utc::now();
    assert_eq!(
        alerts.update(&devices, Thresholds::default(), now),
        vec![Kind::Mouse]
    );
    assert!(
        alerts
            .update(&devices, Thresholds::default(), now)
            .is_empty()
    );
    devices.get_mut(&Kind::Mouse).unwrap().percentage = 30;
    assert!(
        alerts
            .update(&devices, Thresholds::default(), now)
            .is_empty()
    );
    devices.get_mut(&Kind::Mouse).unwrap().percentage = 31;
    alerts.update(&devices, Thresholds::default(), now);
    devices.get_mut(&Kind::Mouse).unwrap().percentage = 10;
    assert_eq!(
        alerts.update(&devices, Thresholds::default(), now),
        vec![Kind::Mouse]
    );
    let mut fresh = Alerts::default();
    devices.get_mut(&Kind::Mouse).unwrap().connected = false;
    assert!(
        fresh
            .update(&devices, Thresholds::default(), now)
            .is_empty()
    );
    devices.get_mut(&Kind::Mouse).unwrap().connected = true;
    devices.get_mut(&Kind::Mouse).unwrap().time = Some(now - chrono::Duration::hours(2));
    assert!(
        fresh
            .update(&devices, Thresholds::default(), now)
            .is_empty()
    );
}

#[test]
fn icons_transparent_theme_and_level_pixels() {
    for size in [16, 20, 24, 32, 48] {
        for kind in Kind::ALL {
            let unknown = icons::tray_rgba(kind, None, size, true, 15, 30);
            assert_eq!(unknown.len(), (size * size * 4) as usize);
            assert!(unknown.chunks(4).any(|p| p[3] == 0));
            assert!(unknown.chunks(4).all(|p| p[3] <= 110));
            let red = icons::tray_rgba(kind, Some(15), size, true, 15, 30);
            assert!(
                red.chunks(4)
                    .any(|p| p[0] == 255 && p[1] < 150 && p[2] < 150 && p[3] > 100)
            );
            let light = icons::tray_rgba(kind, Some(100), size, false, 15, 30);
            assert!(light.chunks(4).any(|p| p == [32, 32, 32, 255]));
        }
    }
}

#[test]
fn options_validate_thresholds_interval_and_unicode_paths() {
    let parse = |args: &[&str]| Options::parse(args.iter().map(OsString::from));
    let defaults = parse(&[]).unwrap();
    assert_eq!(defaults.interval, 30);
    assert_eq!(defaults.thresholds.critical, 15);
    let options = parse(&[
        "--db",
        "C:/test path/şarj.db",
        "--interval",
        "5",
        "--low",
        "50",
        "--critical",
        "20",
        "--sandbox",
    ])
    .unwrap();
    assert_eq!(options.db.to_str().unwrap(), "C:/test path/şarj.db");
    assert_eq!(options.interval, 5);
    assert!(options.sandbox);
    for args in [
        vec!["--interval", "0"],
        vec!["--interval", "86401"],
        vec!["--low", "101"],
        vec!["--critical", "31"],
        vec!["--db"],
        vec!["--wat"],
        vec!["--instance", "bad/name"],
    ] {
        assert!(parse(&args).is_err(), "{args:?}");
    }
}
