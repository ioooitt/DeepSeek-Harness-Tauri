use std::{io::{BufRead, BufReader}, path::PathBuf, process::{Child, Command, Stdio}, sync::Mutex, thread};
use std::os::{linux::net::SocketAddrExt, unix::{fs::MetadataExt, net::{SocketAddr, UnixListener, UnixStream}}};
use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};

struct Server(Mutex<Option<Child>>);

fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn main() {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR is required");
    let uid = std::fs::metadata(runtime).expect("cannot read session directory").uid();
    let address = SocketAddr::from_abstract_name(format!("deepseek-harness-tauri-{uid}")).unwrap();
    let listener = match UnixListener::bind_addr(&address) {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            UnixStream::connect_addr(&address).expect("cannot contact existing DeepSeek Harness");
            return;
        }
        Err(error) => panic!("cannot start single-instance listener: {error}"),
    };
    let app = tauri::Builder::default()
        .manage(Server(Mutex::new(None)))
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(move |app| {
            let show = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出 DeepSeek Harness", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::with_id("deepseek-harness")
                .icon(app.default_window_icon().expect("bundled application icon").clone())
                .tooltip("DeepSeek Harness")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    _ => (),
                })
                .build(app)?;
            let handle = app.handle().clone();
            thread::spawn(move || {
                for connection in listener.incoming() {
                    if connection.is_ok() { show_main(&handle); }
                }
            });
            let resources = app.path().resource_dir()?.join("resources/runtime");
            let node = resources.join("node/bin/node");
            let entry = resources.join("harness/node_modules/@deepseek-ai/dsh/lib/bin.js");
            if !node.is_file() || !entry.is_file() {
                return Err(format!("Harness runtime missing in {}", resources.display()).into());
            }
            let mut path = PathBuf::from(resources.join("node/bin")).into_os_string();
            path.push(":");
            path.push(std::env::var_os("PATH").unwrap_or_default());
            let mut child = Command::new(node).arg(entry)
                .args(["web", "--host", "127.0.0.1", "--port", "0", "--no-open"])
                .env("PATH", path).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn()?;
            let stdout = child.stdout.take().ok_or("No server stdout")?;
            *app.state::<Server>().0.lock().unwrap() = Some(child);
            let handle = app.handle().clone();
            thread::spawn(move || {
                let mut launched = false;
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if let Some(url) = line.trim().strip_prefix("dsh web: ") {
                        if launched { continue; }
                        if !url.starts_with("http://127.0.0.1:") { continue; }
                        if let Ok(parsed) = url.parse::<tauri::Url>() {
                            let main = WebviewWindowBuilder::new(&handle, "main", WebviewUrl::External(parsed))
                                .title("DeepSeek Harness")
                                .inner_size(1200.0, 800.0)
                                .min_inner_size(800.0, 600.0)
                                .build();
                            if let Err(error) = main {
                                eprintln!("Failed to open local Harness: {error}");
                            }
                            launched = true;
                        }
                    }
                }
            });
            Ok(())
        }).build(tauri::generate_context!()).expect("failed to start DeepSeek Harness");
    app.run(|handle, event| {
        if matches!(event, RunEvent::Exit) {
            if let Some(mut child) = handle.state::<Server>().0.lock().unwrap().take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    });
}
