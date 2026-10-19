#![cfg(windows)]

use logibar::icons::Kind;
use rusqlite::Connection;
use serde_json::Value;
use std::{
    fs,
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::RECT,
    UI::{
        Shell::{NOTIFYICONIDENTIFIER, Shell_NotifyIconGetRect},
        WindowsAndMessaging::{PostMessageW, WM_APP, WM_CLOSE, WM_RBUTTONUP},
    },
};

fn wait_report(path: &std::path::Path) -> Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(bytes) = fs::read(path)
            && let Ok(value) = serde_json::from_slice::<Value>(&bytes)
            && value["status"] == "running"
        {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "No running report at {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(100));
    }
}

fn wait_exit(child: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            return;
        }
        assert!(
            Instant::now() < deadline,
            "Application failed to exit gracefully"
        );
        thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn native_tray_shell_registration_refresh_single_instance_and_graceful_close() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("settings.db");
    let report = dir.path().join("runtime.json");
    let connection = Connection::open(&db).unwrap();
    connection
        .execute_batch("CREATE TABLE data (_id INTEGER PRIMARY KEY, file BLOB NOT NULL);")
        .unwrap();
    connection
        .execute(
            "INSERT INTO data VALUES (1, ?1)",
            [include_str!("fixtures/settings.json")],
        )
        .unwrap();
    let instance = format!("test-{}", std::process::id());
    let mut child = Command::new(env!("CARGO_BIN_EXE_logibar"))
        .args([
            "--sandbox",
            "--instance",
            &instance,
            "--interval",
            "1",
            "--exit-after",
            "30",
            "--db",
        ])
        .arg(&db)
        .arg("--report")
        .arg(&report)
        .spawn()
        .unwrap();
    let initial = wait_report(&report);
    println!("initial native shell report: {initial}");
    assert_eq!(initial["pid"], child.id());
    assert_eq!(initial["adds"], 3);
    let hwnd = initial["hwnd"].as_u64().unwrap() as usize as *mut std::ffi::c_void;
    for (index, kind) in Kind::ALL.into_iter().enumerate() {
        assert_eq!(initial["tray"][index]["kind"], kind.name());
        assert_eq!(initial["tray"][index]["registered"], true);
        let identifier = NOTIFYICONIDENTIFIER {
            cbSize: std::mem::size_of::<NOTIFYICONIDENTIFIER>() as u32,
            hWnd: hwnd,
            uID: index as u32 + 1,
            ..unsafe { std::mem::zeroed() }
        };
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        assert!(
            unsafe { Shell_NotifyIconGetRect(&identifier, &mut rect) } >= 0,
            "Shell cannot find {} icon",
            kind.name()
        );
        assert!(rect.right > rect.left && rect.bottom > rect.top);
    }
    let duplicate = Command::new(env!("CARGO_BIN_EXE_logibar"))
        .args(["--sandbox", "--instance", &instance])
        .status()
        .unwrap();
    assert!(duplicate.success());
    assert!(child.try_wait().unwrap().is_none());
    connection
        .execute("INSERT INTO data VALUES (2, '{')", [])
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let value = wait_report(&report);
        if value["error"].is_string() {
            assert_eq!(value["tray"][0]["reading"]["percentage"], 70);
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(100));
    }
    connection
        .execute("INSERT INTO data VALUES (3, '{}')", [])
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let value = wait_report(&report);
        if value["tray"][0]["reading"].is_null() && value["removes"] == 1 {
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(100));
    }
    assert_ne!(unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) }, 0);
    wait_exit(&mut child);
    let final_report: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    println!("final native shell report: {final_report}");
    assert_eq!(final_report["status"], "exited");
    assert_eq!(final_report["adds"], 3);
    assert_eq!(final_report["removes"], 3);
    assert!(final_report["refreshes"].as_u64().unwrap() >= 2);
    assert!(
        final_report["tray"]
            .as_array()
            .unwrap()
            .iter()
            .all(|icon| icon["registered"] == false && icon["shell_confirmed"] == false)
    );
}

#[test]
fn missing_database_shows_two_unknown_icons_and_timer_exits_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("missing.json");
    let missing = dir.path().join("missing.db");
    let instance = format!("missing-{}", std::process::id());
    let mut child = Command::new(env!("CARGO_BIN_EXE_logibar"))
        .args([
            "--sandbox",
            "--instance",
            &instance,
            "--exit-after",
            "2",
            "--db",
        ])
        .arg(&missing)
        .arg("--report")
        .arg(&report)
        .spawn()
        .unwrap();
    let running = wait_report(&report);
    assert_eq!(running["adds"], 2);
    assert!(running["tray"][0]["reading"].is_null());
    assert!(running["tray"][1]["reading"].is_null());
    assert_eq!(running["tray"][2]["registered"], false);
    let hwnd = running["hwnd"].as_u64().unwrap() as usize as *mut std::ffi::c_void;
    assert_ne!(
        unsafe { PostMessageW(hwnd, WM_APP + 1, 0, WM_RBUTTONUP as isize) },
        0
    );
    wait_exit(&mut child);
    let exited: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    assert_eq!(exited["status"], "exited");
    assert_eq!(exited["removes"], 2);
    assert!(!missing.exists());
    println!("missing database timer exit: {exited}");
}

#[test]
fn inspect_reads_native_sqlite_without_tray_or_startup() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("settings.db");
    let report = dir.path().join("inspect.json");
    let connection = Connection::open(&db).unwrap();
    connection
        .execute_batch("CREATE TABLE data (_id INTEGER PRIMARY KEY, file BLOB NOT NULL);")
        .unwrap();
    connection
        .execute(
            "INSERT INTO data VALUES (1, ?1)",
            [include_str!("fixtures/settings.json")],
        )
        .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_logibar"))
        .args(["--inspect", "--db"])
        .arg(db)
        .arg("--report")
        .arg(&report)
        .status()
        .unwrap();
    assert!(status.success());
    let readings: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    assert_eq!(readings.as_array().unwrap().len(), 3);
    assert_eq!(readings[0]["percentage"], 70);
}
