use crate::battery::Thresholds;
use std::{ffi::OsString, path::PathBuf};

pub struct Options {
    pub db: PathBuf,
    pub interval: u32,
    pub thresholds: Thresholds,
    pub instance: String,
    pub sandbox: bool,
    pub exit_after: Option<u32>,
    pub report: Option<PathBuf>,
    pub inspect: bool,
    pub help: bool,
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut options = Self {
            db: std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_default()
                .join("LGHUB/settings.db"),
            interval: 30,
            thresholds: Thresholds::default(),
            instance: "default".into(),
            sandbox: false,
            exit_after: None,
            report: None,
            inspect: false,
            help: false,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg.to_str().ok_or("Invalid option encoding")?;
            match arg {
                "--sandbox" => options.sandbox = true,
                "--inspect" => options.inspect = true,
                "--help" | "-h" => options.help = true,
                "--db" => options.db = PathBuf::from(args.next().ok_or("--db needs a path")?),
                "--report" => {
                    options.report =
                        Some(PathBuf::from(args.next().ok_or("--report needs a path")?))
                }
                "--instance" => {
                    options.instance = args
                        .next()
                        .ok_or("--instance needs a name")?
                        .into_string()
                        .map_err(|_| "Invalid instance")?;
                    if options.instance.is_empty()
                        || !options
                            .instance
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-')
                    {
                        return Err("Instance must contain only letters, digits or hyphens".into());
                    }
                }
                "--interval" | "--critical" | "--low" | "--exit-after" => {
                    let value: u32 = args
                        .next()
                        .ok_or_else(|| format!("{arg} needs a number"))?
                        .to_str()
                        .ok_or("Invalid number")?
                        .parse()
                        .map_err(|_| format!("Invalid number for {arg}"))?;
                    match arg {
                        "--interval" if (1..=86400).contains(&value) => options.interval = value,
                        "--critical" if value <= 100 => options.thresholds.critical = value as u8,
                        "--low" if value <= 100 => options.thresholds.low = value as u8,
                        "--exit-after" if (1..=86400).contains(&value) => {
                            options.exit_after = Some(value)
                        }
                        _ => return Err(format!("Out-of-range value for {arg}")),
                    }
                }
                _ => return Err(format!("Unknown option: {arg}")),
            }
        }
        if options.thresholds.critical > options.thresholds.low {
            return Err("Critical threshold must not exceed low threshold".into());
        }
        Ok(options)
    }
}

pub const HELP: &str = "logibar 2.0\n\n--db PATH          G HUB settings.db (default: %LOCALAPPDATA%/LGHUB/settings.db)\n--interval SECONDS Refresh every 1..86400 seconds (default: 30)\n--critical PERCENT Critical threshold (default: 15)\n--low PERCENT      Low threshold (default: 30)\n--sandbox          Disable startup changes and battery notifications\n--instance NAME    Separate instance for testing\n--exit-after SECS  Exit gracefully after a bounded smoke test\n--report PATH      Write local diagnostic JSON (device names and readings)\n--inspect          Read database once and write report without creating tray icons\n--help             Show these options\n";
