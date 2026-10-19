use chrono::Utc;
use logibar::{
    battery::{self, Alerts, Devices},
    icons::{self, Kind},
    options::{HELP, Options},
};
use serde_json::json;
use std::{
    ffi::OsStr,
    mem::{size_of, zeroed},
    os::windows::ffi::OsStrExt,
    ptr::{null, null_mut},
    sync::mpsc,
};
use windows_sys::Win32::{
    Foundation::*,
    System::{LibraryLoader::GetModuleHandleW, Registry::*, Threading::CreateMutexW},
    UI::{HiDpi::*, Shell::*, WindowsAndMessaging::*},
};

const CALLBACK: u32 = WM_APP + 1;
const NIN_KEYSELECT: u32 = NIN_SELECT | 1;
const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const APP_ID: &str = "menzo.logibar";

fn wide(text: impl AsRef<OsStr>) -> Vec<u16> {
    text.as_ref().encode_wide().chain(Some(0)).collect()
}

fn copy_wide(dest: &mut [u16], text: &str) {
    dest.fill(0);
    let mut index = 0;
    for character in text.chars() {
        let mut buffer = [0; 2];
        let encoded = character.encode_utf16(&mut buffer);
        if index + encoded.len() >= dest.len() {
            break;
        }
        dest[index..index + encoded.len()].copy_from_slice(encoded);
        index += encoded.len();
    }
}

fn registry_string(key: &str, name: &str) -> Option<String> {
    unsafe {
        let mut bytes = 0;
        if RegGetValueW(
            HKEY_CURRENT_USER,
            wide(key).as_ptr(),
            wide(name).as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            null_mut(),
            &mut bytes,
        ) != 0
        {
            return None;
        }
        let mut value = vec![0u16; bytes as usize / 2];
        if RegGetValueW(
            HKEY_CURRENT_USER,
            wide(key).as_ptr(),
            wide(name).as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            value.as_mut_ptr().cast(),
            &mut bytes,
        ) != 0
        {
            return None;
        }
        Some(
            String::from_utf16_lossy(&value)
                .trim_end_matches('\0')
                .to_string(),
        )
    }
}

fn dark_theme() -> bool {
    unsafe {
        let mut value = 0u32;
        let mut size = 4;
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize").as_ptr(),
            wide("SystemUsesLightTheme").as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            (&mut value as *mut u32).cast(),
            &mut size,
        ) != 0
            || value == 0
    }
}

fn startup_command(options: &Options) -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(format!(
        "\"{}\" --db \"{}\" --interval {} --critical {} --low {}",
        exe.display(),
        options.db.display(),
        options.interval,
        options.thresholds.critical,
        options.thresholds.low
    ))
}

fn startup_enabled() -> bool {
    let Some(command) = registry_string(RUN_KEY, "logibar") else {
        return false;
    };
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    command == format!("\"{}\"", exe.display())
        || command.starts_with(&format!("\"{}\" ", exe.display()))
}

fn toggle_startup(options: &Options) -> Result<(), String> {
    if options.sandbox {
        return Ok(());
    }
    unsafe {
        let result = if startup_enabled() {
            RegDeleteKeyValueW(
                HKEY_CURRENT_USER,
                wide(RUN_KEY).as_ptr(),
                wide("logibar").as_ptr(),
            )
        } else {
            let command = wide(startup_command(options)?);
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                wide(RUN_KEY).as_ptr(),
                wide("logibar").as_ptr(),
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * 2) as u32,
            )
        };
        if result != 0 {
            return Err(format!(
                "Cannot change startup entry (Windows error {result})"
            ));
        }
    }
    Ok(())
}

enum Event {
    Refresh,
    Menu,
    Quit,
    Theme,
    Taskbar,
}
struct Context {
    sender: mpsc::Sender<Event>,
    taskbar: u32,
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // The context stays boxed until DestroyWindow returns. Callbacks only queue events,
    // so TrackPopupMenu's nested message loop cannot alias the mutable application state.
    unsafe {
        let context = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Context;
        if !context.is_null() {
            let event = match message {
                WM_CLOSE => Some(Event::Quit),
                WM_TIMER if wparam == 2 => Some(Event::Quit),
                WM_TIMER => Some(Event::Refresh),
                WM_SETTINGCHANGE | WM_DPICHANGED | WM_DISPLAYCHANGE => Some(Event::Theme),
                CALLBACK
                    if [
                        WM_LBUTTONUP,
                        WM_RBUTTONUP,
                        WM_CONTEXTMENU,
                        NIN_SELECT,
                        NIN_KEYSELECT,
                    ]
                    .contains(&(lparam as u32 & 0xffff)) =>
                {
                    Some(Event::Menu)
                }
                _ if message == (*context).taskbar => Some(Event::Taskbar),
                _ => None,
            };
            if let Some(event) = event {
                if matches!(event, Event::Quit) {
                    EndMenu();
                }
                let _ = (*context).sender.send(event);
                // Sent messages (tray callbacks, broadcasts) are handled inside GetMessageW
                // without returning, so wake the loop to drain the queue.
                PostMessageW(hwnd, WM_NULL, 0, 0);
                return 0;
            }
        }
        if message == WM_DESTROY {
            PostQuitMessage(0);
            return 0;
        }
        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}

struct App {
    hwnd: HWND,
    options: Options,
    devices: Devices,
    error: Option<String>,
    alerts: Alerts,
    handles: [HICON; 3],
    added: [bool; 3],
    adds: usize,
    removes: usize,
    refreshes: usize,
}

impl App {
    fn notify_data(&self, index: usize) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: index as u32 + 1,
            ..unsafe { zeroed() }
        }
    }

    fn refresh(&mut self) -> Result<(), String> {
        match battery::read_devices(&self.options.db) {
            Ok(devices) => {
                self.devices = devices;
                self.error = None;
            }
            Err(error) => self.error = Some(format!("G HUB read failed: {error}")),
        }
        self.refreshes += 1;
        self.redraw()?;
        if self.error.is_none() && !self.options.sandbox {
            for kind in self
                .alerts
                .update(&self.devices, self.options.thresholds, Utc::now())
            {
                let index = Kind::ALL.iter().position(|&k| k == kind).unwrap();
                let mut data = self.notify_data(index);
                data.uFlags = NIF_INFO;
                data.dwInfoFlags = NIIF_WARNING;
                copy_wide(&mut data.szInfoTitle, "Low battery");
                copy_wide(
                    &mut data.szInfo,
                    &format!(
                        "{} is at {}%. Time to charge.",
                        self.devices[&kind].name, self.devices[&kind].percentage
                    ),
                );
                unsafe {
                    Shell_NotifyIconW(NIM_MODIFY, &data);
                }
            }
        }
        self.report("running")
    }

    fn redraw(&mut self) -> Result<(), String> {
        unsafe {
            let size = GetSystemMetrics(SM_CXSMICON).max(16) as u32;
            for (index, kind) in Kind::ALL.into_iter().enumerate() {
                let reading = self.devices.get(&kind);
                let visible = kind != Kind::Keyboard || reading.is_some();
                if !visible {
                    self.remove(index);
                    continue;
                }
                let percentage = reading.filter(|r| r.connected).map(|r| r.percentage);
                let rgba = icons::tray_rgba(
                    kind,
                    percentage,
                    size,
                    dark_theme(),
                    self.options.thresholds.critical,
                    self.options.thresholds.low,
                );
                let image = icons::ico(&[(size, rgba)]);
                let handle = CreateIconFromResourceEx(
                    image[22..].as_ptr(),
                    (image.len() - 22) as u32,
                    1,
                    0x00030000,
                    size as i32,
                    size as i32,
                    0,
                );
                if handle.is_null() {
                    return Err(format!(
                        "Cannot create {} tray icon: {}",
                        kind.name(),
                        std::io::Error::last_os_error()
                    ));
                }
                let mut data = self.notify_data(index);
                data.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE | NIF_SHOWTIP;
                data.uCallbackMessage = CALLBACK;
                data.hIcon = handle;
                let mut tooltip = reading
                    .map(|r| r.label(Utc::now()))
                    .unwrap_or_else(|| format!("{}: N/A. Is G HUB running?", kind.name()));
                if self.error.is_some() {
                    tooltip.push_str(" (G HUB unavailable; cached)");
                }
                copy_wide(&mut data.szTip, &tooltip);
                if Shell_NotifyIconW(
                    if self.added[index] {
                        NIM_MODIFY
                    } else {
                        NIM_ADD
                    },
                    &data,
                ) == 0
                {
                    // Explorer may be busy or restarting; retry on the next refresh.
                    DestroyIcon(handle);
                    self.added[index] = false;
                    continue;
                }
                if !self.added[index] {
                    self.added[index] = true;
                    self.adds += 1;
                    data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
                    Shell_NotifyIconW(NIM_SETVERSION, &data);
                }
                if !self.handles[index].is_null() {
                    DestroyIcon(self.handles[index]);
                }
                self.handles[index] = handle;
            }
        }
        Ok(())
    }

    fn remove(&mut self, index: usize) {
        unsafe {
            if self.added[index] {
                if Shell_NotifyIconW(NIM_DELETE, &self.notify_data(index)) != 0 {
                    self.removes += 1;
                }
                self.added[index] = false;
            }
            if !self.handles[index].is_null() {
                DestroyIcon(self.handles[index]);
                self.handles[index] = null_mut();
            }
        }
    }

    fn menu(&mut self) -> Result<bool, String> {
        unsafe {
            let menu = CreatePopupMenu();
            if menu.is_null() {
                return Err("Cannot create tray menu".into());
            }
            for reading in self.devices.values() {
                AppendMenuW(
                    menu,
                    MF_STRING | MF_DISABLED,
                    0,
                    wide(reading.label(Utc::now())).as_ptr(),
                );
            }
            if self.devices.is_empty() {
                AppendMenuW(
                    menu,
                    MF_STRING | MF_DISABLED,
                    0,
                    wide("No devices found. Is G HUB running?").as_ptr(),
                );
            }
            if let Some(error) = &self.error {
                AppendMenuW(menu, MF_STRING | MF_DISABLED, 0, wide(error).as_ptr());
            }
            AppendMenuW(menu, MF_SEPARATOR, 0, null());
            AppendMenuW(menu, MF_STRING, 1, wide("Refresh").as_ptr());
            let flags = MF_STRING
                | if self.options.sandbox {
                    MF_DISABLED
                } else if startup_enabled() {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                };
            AppendMenuW(menu, flags, 2, wide("Start with Windows").as_ptr());
            AppendMenuW(menu, MF_SEPARATOR, 0, null());
            AppendMenuW(menu, MF_STRING, 3, wide("Quit").as_ptr());
            let mut point = POINT { x: 0, y: 0 };
            GetCursorPos(&mut point);
            SetForegroundWindow(self.hwnd);
            let command = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                point.x,
                point.y,
                0,
                self.hwnd,
                null(),
            );
            PostMessageW(self.hwnd, WM_NULL, 0, 0);
            DestroyMenu(menu);
            match command {
                1 => self.refresh()?,
                2 => {
                    if let Err(error) = toggle_startup(&self.options) {
                        show_error(&error);
                    }
                }
                3 => return Ok(false),
                _ => (),
            }
        }
        Ok(true)
    }

    fn report(&self, status: &str) -> Result<(), String> {
        let Some(path) = &self.options.report else {
            return Ok(());
        };
        let tray: Vec<_> = Kind::ALL.into_iter().enumerate().map(|(index, kind)| {
            let mut rect: RECT = unsafe { zeroed() };
            let identifier = NOTIFYICONIDENTIFIER { cbSize: size_of::<NOTIFYICONIDENTIFIER>() as u32, hWnd: self.hwnd, uID: index as u32 + 1, ..unsafe { zeroed() } };
            let found = unsafe { Shell_NotifyIconGetRect(&identifier, &mut rect) } >= 0;
            let reading = self.devices.get(&kind);
            json!({ "kind": kind.name(), "registered": self.added[index], "shell_confirmed": found,
                "rect": [rect.left, rect.top, rect.right, rect.bottom],
                "reading": reading.map(|r| json!({ "name": r.name, "percentage": r.percentage, "connected": r.connected, "label": r.label(Utc::now()) })) })
        }).collect();
        let report = json!({ "version": env!("CARGO_PKG_VERSION"), "pid": std::process::id(), "hwnd": self.hwnd as usize,
            "status": status, "sandbox": self.options.sandbox, "refreshes": self.refreshes,
            "adds": self.adds, "removes": self.removes, "error": self.error, "tray": tray });
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("Cannot write diagnostic report: {e}"))
    }
}

impl Drop for App {
    fn drop(&mut self) {
        for index in 0..3 {
            self.remove(index);
        }
    }
}

pub fn show_error(error: &str) {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(error).as_ptr(),
            wide("logibar").as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

pub fn run() -> Result<(), String> {
    let options = Options::parse(std::env::args_os().skip(1))?;
    if options.help {
        unsafe {
            MessageBoxW(
                null_mut(),
                wide(HELP).as_ptr(),
                wide("logibar").as_ptr(),
                MB_OK,
            );
        }
        return Ok(());
    }
    if options.inspect {
        let devices = battery::read_devices(&options.db).map_err(|e| e.to_string())?;
        let report = options
            .report
            .as_ref()
            .ok_or("--inspect requires --report PATH")?;
        let readings: Vec<_> = devices.iter().map(|(kind, r)| json!({"kind": kind.name(), "name": r.name, "percentage": r.percentage, "connected": r.connected, "label": r.label(Utc::now())})).collect();
        return std::fs::write(
            report,
            serde_json::to_vec_pretty(&readings).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string());
    }
    unsafe {
        SetLastError(0);
        let mutex = CreateMutexW(
            null(),
            0,
            wide(format!("Local\\{APP_ID}.{}", options.instance)).as_ptr(),
        );
        if mutex.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(mutex);
            return Ok(());
        }
        let result = run_window(options);
        CloseHandle(mutex);
        result
    }
}

fn run_window(options: Options) -> Result<(), String> {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        SetCurrentProcessExplicitAppUserModelID(wide(APP_ID).as_ptr());
        let module = GetModuleHandleW(null());
        let class_name = wide(format!("logibar-{}", options.instance));
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: module,
            lpszClassName: class_name.as_ptr(),
            hIcon: LoadIconW(module, std::ptr::without_provenance(1)),
            ..zeroed()
        };
        if RegisterClassW(&class) == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            module,
            null(),
        );
        if hwnd.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let (sender, receiver) = mpsc::channel();
        let context = Box::new(Context {
            sender,
            taskbar: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
        });
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&*context as *const Context) as isize);
        let mut app = App {
            hwnd,
            options,
            devices: Devices::new(),
            error: None,
            alerts: Alerts::default(),
            handles: [null_mut(); 3],
            added: [false; 3],
            adds: 0,
            removes: 0,
            refreshes: 0,
        };
        let result = (|| {
            app.refresh()?;
            if SetTimer(hwnd, 1, app.options.interval * 1000, None) == 0 {
                return Err("Cannot create refresh timer".into());
            }
            if let Some(seconds) = app.options.exit_after
                && SetTimer(hwnd, 2, seconds * 1000, None) == 0
            {
                return Err("Cannot create exit timer".into());
            }
            let mut message: MSG = zeroed();
            'messages: loop {
                let result = GetMessageW(&mut message, null_mut(), 0, 0);
                if result == -1 {
                    return Err(std::io::Error::last_os_error().to_string());
                }
                if result == 0 {
                    break;
                }
                TranslateMessage(&message);
                DispatchMessageW(&message);
                while let Ok(event) = receiver.try_recv() {
                    match event {
                        Event::Quit => break 'messages,
                        Event::Refresh => app.refresh()?,
                        Event::Theme => {
                            app.redraw()?;
                            app.report("running")?;
                        }
                        Event::Taskbar => {
                            app.added.fill(false);
                            app.redraw()?;
                        }
                        Event::Menu => {
                            if !app.menu()? {
                                break 'messages;
                            }
                        }
                    }
                }
            }
            Ok(())
        })();
        KillTimer(hwnd, 1);
        KillTimer(hwnd, 2);
        for index in 0..3 {
            app.remove(index);
        }
        let report = app.report("exited");
        DestroyWindow(hwnd);
        drop(context);
        result.and(report)
    }
}
