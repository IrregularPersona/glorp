#![cfg_attr(feature = "packaged", windows_subsystem = "windows")]
use crate::{modules::userscripts, window::WindowState};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};
use std::{
    collections::HashMap,
    env, fs, io, path, process, result, sync,
    sync::{Arc, LazyLock, Mutex},
};
use webview2_com::{Microsoft::Web::WebView2::Win32::*, *};
use windows::{
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
    core::*,
};

mod config;
mod constants;
mod utils;
mod window;
mod modules {
    pub mod blocklist;
    pub mod flaglist;
    pub mod lifecycle;
    pub mod ping;
    pub mod priority;
    pub mod swapper;
    pub mod userscripts;
}

static LAUNCH_ARGS: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(env::args().skip(1).collect()));
static CONFIG: LazyLock<Mutex<config::Config>> = LazyLock::new(|| Mutex::new(config::Config::load()));
static JS_VERSION: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new("0.0.0".to_string()));
static SCRIPT_ID: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new(String::new()));
static mut TOKEN: *mut i64 = &mut 0i64 as *mut i64;

fn config_bool(setting: &str, default: bool) -> bool {
    CONFIG.lock().unwrap().get(setting).unwrap_or(default)
}

fn config_string(setting: &str, default: impl Into<String>) -> String {
    CONFIG
        .lock()
        .unwrap()
        .get::<String>(setting)
        .unwrap_or_else(|| default.into())
}

fn parse_web_message_value(value: &str) -> serde_json::Value {
    if let Ok(bool_val) = value.parse::<bool>() {
        serde_json::Value::Bool(bool_val)
    } else if let Ok(int_val) = value.parse::<i64>() {
        serde_json::Value::Number(serde_json::Number::from(int_val))
    } else if let Ok(float_val) = value.parse::<f64>() {
        serde_json::Value::Number(
            serde_json::Number::from_f64((float_val * 100.0).round() / 100.0).unwrap(),
        )
    } else {
        serde_json::Value::String(value.to_string())
    }
}

fn settings_dir() -> path::PathBuf {
    path::PathBuf::from(env::var("USERPROFILE").unwrap())
        .join("Documents")
        .join("glorp")
}

fn open_documents_path(path: path::PathBuf) {
    process::Command::new("explorer.exe").arg(path).spawn().ok();
}

fn init_fs() -> result::Result<(), io::Error> {
    let user_profile = path::PathBuf::from(env::var("USERPROFILE").unwrap());
    let client_dir = user_profile.join("Documents").join("glorp");
    let swap_dir = client_dir.join("swapper");
    let scripts_dir = client_dir.join("scripts").join("social");
    let flaglist_path = client_dir.join("user_flags.json");
    let blocklist_path = client_dir.join("user_blocklist.json");

    let resources_dir = env::current_exe().unwrap().parent().unwrap().join("resources");

    fs::create_dir_all(&swap_dir)?;
    fs::create_dir_all(&scripts_dir)?;
    fs::create_dir_all(&resources_dir)?;

    if !path::Path::new(&flaglist_path).exists() {
        fs::write(&flaglist_path, constants::DEFAULT_FLAGS)?;
    }
    if !path::Path::new(&blocklist_path).exists() {
        fs::write(&blocklist_path, constants::DEFAULT_BLOCKLIST)?;
    }
    Ok(())
}

fn set_permission_requested_handler(webview: &ICoreWebView2, token: &mut i64) {
    unsafe {
        webview
            .add_PermissionRequested(
                &PermissionRequestedEventHandler::create(Box::new(
                    move |_, args: Option<ICoreWebView2PermissionRequestedEventArgs>| {
                        args.unwrap().SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW).ok();
                        Ok(())
                    },
                )),
                token,
            )
            .ok();
    }
}

fn set_web_resource_requested_handler(webview: &ICoreWebView2, env: &ICoreWebView2Environment, token: &mut i64) {
    let env_clone = env.clone();
    let swaps = if config_bool("swapper", true) {
        modules::swapper::load(webview)
    } else {
        HashMap::new()
    };

    unsafe {
        webview
            .add_WebResourceRequested(
                &WebResourceRequestedEventHandler::create(Box::new(move |webview, args| {
                    let Some(args) = args else {
                        return Ok(());
                    };
                    let request: ICoreWebView2WebResourceRequest = args.Request()?;
                    let mut uri = PWSTR::null();
                    request.Uri(&mut uri)?;
                    let uri = take_pwstr(uri);

                    if uri.contains("krunker.io") {
                        if uri.contains("game-info") || uri.contains("lobby-ranked") {
                            webview.unwrap().PostWebMessageAsString(w!("game-updated")).ok();
                            return Ok(());
                        }

                        let filename: &str = uri
                            .split("krunker.io/")
                            .nth(1)
                            .and_then(|s| s.split('?').next())
                            .unwrap_or("");

                        if let Some(stream) = swaps.get(filename) {
                            let response = env_clone.CreateWebResourceResponse(
                                stream,
                                200,
                                w!("OK"),
                                w!("Access-Control-Allow-Origin: *"),
                            )?;
                            args.SetResponse(Some(&response))?;
                            return Ok(());
                        }
                    }

                    request.SetUri(PCWSTR::null())?;
                    Ok(())
                })),
                token,
            )
            .ok();
    }
}

fn set_new_window_requested_handler(webview: &ICoreWebView2, env: &ICoreWebView2Environment, token: &mut i64) {
    // let the clone happens outside unsafe.
    // lets just not have the clone happen inside unsafe xddd
    let env_clone = env.clone();
    unsafe {
        webview
            .add_NewWindowRequested(
                &NewWindowRequestedEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else {
                        return Ok(());
                    };
                    let features = args.WindowFeatures()?;
                    let mut has_position: BOOL = false.into();
                    let _ = features.HasPosition(&mut has_position);
                    let mut has_size: BOOL = false.into();
                    let _ = features.HasSize(&mut has_size);
                    let mut window_state = None;
                    if has_position.as_bool() && has_size.as_bool() {
                        let mut left = 0;
                        let mut top = 0;
                        let mut width = 0;
                        let mut height = 0;
                        let _ = features.Left(&mut left);
                        let _ = features.Top(&mut top);
                        let _ = features.Width(&mut width);
                        let _ = features.Height(&mut height);
                        window_state = Some(WindowState {
                            fullscreen: false,
                            position: window::Position {
                                left: left as i32,
                                top: top as i32,
                                right: left as i32 + width as i32,
                                bottom: top as i32 + height as i32,
                            },
                        });
                    }

                    let deferral = args.GetDeferral()?;
                    args.SetHandled(true).unwrap();
                    let (hwnd, window_state) = window::create_window("Custom", true, window_state);
                    let mut uri = PWSTR::null();
                    let _ = args.Uri(&mut uri);
                    let uri = take_pwstr(uri);
                    let args = utils::UnsafeSend::new(args);
                    let deferral = utils::UnsafeSend::new(deferral);
                    let env_for_creation = env_clone.clone();
                    let env_for_handler = utils::UnsafeSend::new(env_clone.clone());
                    window::create_core_webview2_controller_async(
                        hwnd,
                        env_for_creation,
                        window_state,
                        move |controller| {
                            let controller = controller.unwrap();
                            let webview = controller.CoreWebView2().unwrap();
                            if uri.contains("krunker.io/social.html")
                                && config_bool("userscripts", false)
                                && let Err(e) = userscripts::load(&webview, true)
                            {
                                println!("can't load userscripts on social window {}", e);
                            }

                            args.take().SetNewWindow(&webview).unwrap();
                            set_handlers(&webview, &env_for_handler);

                            deferral.take().Complete().ok();
                        },
                    );

                    Ok(())
                })),
                token,
            )
            .ok();
    }
}

fn set_handlers<T: utils::EnvironmentRef>(webview: &ICoreWebView2, env_wrapper: &T) {
    let env: &ICoreWebView2Environment = env_wrapper.env_ref();
    let mut token = 0i64;

    set_permission_requested_handler(webview, &mut token);

    if config_bool("blocklist", true) {
        modules::blocklist::load(webview);
    }

    set_web_resource_requested_handler(webview, env, &mut token);
    set_new_window_requested_handler(webview, env, &mut token);
}

fn apply_hard_flip(webview2_folder: &path::Path) {
    if config_bool("hardFlip", true) {
        fs::rename(
            webview2_folder.join("OLD_vk_swiftshader.dll"),
            webview2_folder.join("vk_swiftshader.dll"),
        )
        .ok();
    } else {
        fs::rename(
            webview2_folder.join("vk_swiftshader.dll"),
            webview2_folder.join("OLD_vk_swiftshader.dll"),
        )
        .ok();
    }
}

fn build_webview_args() -> String {
    let mut args = modules::flaglist::load();
    if config_bool("uncapFps", true) {
        args.push_str(" --disable-frame-rate-limit");
    }
    args
}

fn create_discord_client_if_enabled() -> Arc<Mutex<Option<DiscordIpcClient>>> {
    let discord_client: Arc<Mutex<Option<DiscordIpcClient>>> = Arc::new(Mutex::new(None));
    if config_bool("discordRPC", false) {
        let mut client = DiscordIpcClient::new(constants::DISCORD_CLIENT_ID);
        client.connect().ok();
        *discord_client.lock().unwrap() = Some(client);
    }
    discord_client
}

fn load_js_bundle() -> String {
    let mut buf = include_str!("../target/bundle.js").to_string();

    #[cfg(feature = "packaged")]
    if let Ok(buffer) = modules::lifecycle::read_js_bundle() {
        buf = buffer;
    }

    buf
}

fn send_info(webview: &ICoreWebView2) {
    let version = env!("CARGO_PKG_VERSION");
    let mut info_map = serde_json::Map::new();
    info_map.insert("settings".to_string(), serde_json::json!(&*CONFIG.lock().unwrap()));
    info_map.insert("version".to_string(), serde_json::Value::String(version.to_string()));
    if !LAUNCH_ARGS.lock().unwrap().is_empty() {
        info_map.insert(
            "launchArgs".to_string(),
            serde_json::Value::String(LAUNCH_ARGS.lock().unwrap().join(" ")),
        );
    }

    let info_json = serde_json::to_string_pretty(&info_map).unwrap();
    unsafe {
        webview
            .PostWebMessageAsJson(PCWSTR(utils::create_utf_string(info_json).as_ptr()))
            .ok();
    }
}

fn open_documents_subpath(target: &str) {
    let path_to_open = match target {
        "blocklist" => settings_dir().join("user_blocklist.json"),
        "swapper" => settings_dir().join("swapper"),
        "userscripts" => settings_dir().join("scripts"),
        _ => return,
    };
    open_documents_path(path_to_open);
}

fn handle_web_message(
    webview: &ICoreWebView2,
    main_window: &window::Window,
    discord_client: &Arc<Mutex<Option<DiscordIpcClient>>>,
    message_string: &str,
) -> result::Result<(), windows::core::Error> {
    let parts: Vec<&str> = message_string.split(", ").map(|s| s.trim()).collect();

    match parts.as_slice() {
        ["set-config", setting, value] => {
            CONFIG.lock().unwrap().set(setting, parse_web_message_value(value));
        }
        ["get-info"] => {
            send_info(webview);
        }
        ["drag", value] => {
            const ENABLED: usize = 2;
            const DISABLED: usize = 0;
            let value = value.parse::<bool>().unwrap_or(false);
            unsafe {
                PostMessageW(
                    main_window.widget_wnd,
                    WM_USER,
                    WPARAM(if value { DISABLED } else { ENABLED }),
                    LPARAM(0),
                )
                .ok();
            }
        }
        ["throttle", status] => {
            let setting = if *status == "game" { "throttle" } else { "inMenuThrottle" };
            utils::set_cpu_throttling(webview, CONFIG.lock().unwrap().get::<f32>(setting).unwrap_or(1.0));
        }
        ["close"] => {
            unsafe { PostQuitMessage(0); }
        }
        ["open", target] => {
            open_documents_subpath(target);
        }
        ["rpc-update", part1, part2] => {
            let state = format!("{} on {}", part1, part2);
            if let Some(client) = &mut *discord_client.lock().unwrap() {
                let activity = activity::Activity::new()
                    .details("Krunker")
                    .state(&state)
                    .assets(activity::Assets::new());
                if let Err(e) = client.set_activity(activity) {
                    eprintln!("Failed to set rpc activity: {}", e);
                }
            }
        }
        ["toggle-rboost", value] => {
            const ENABLED: usize = 1;
            const DISABLED: usize = 3;
            let value = value.parse::<bool>().unwrap_or(false);
            unsafe {
                PostMessageW(
                    Some(utils::find_child_window_by_class(
                        FindWindowW(w!("krunker_webview"), PCWSTR::null()).unwrap(),
                        "Chrome_RenderWidgetHostHWND",
                    )),
                    WM_USER,
                    WPARAM(if value { ENABLED } else { DISABLED }),
                    LPARAM(0),
                )
                .ok();
            }
        }
        ["ping"] => {
            modules::ping::ping(webview);
        }
        _ => {}
    }

    Ok(())
}

pub fn create_main_window(env: Option<ICoreWebView2Environment>) -> window::Window {
    let mut webview2_folder: path::PathBuf = env::current_exe().unwrap();
    webview2_folder.pop();
    webview2_folder = webview2_folder.join("WebView2");

    apply_hard_flip(&webview2_folder);

    let args = build_webview_args();
    let start_mode = config_string("startMode", "Remember Previous");
    let state = if start_mode == "Remember Previous" {
        CONFIG.lock().unwrap().get::<window::WindowState>("lastPosition")
    } else {
        None
    };

    let mut main_window = window::Window::new_core(&start_mode, args, env, state);
    let discord_client = create_discord_client_if_enabled();

    modules::priority::set(config_string("webviewPriority", "Normal").as_str());

    if config_bool("userscripts", false)
        && let Err(e) = userscripts::load(&main_window.webview, false)
    {
        eprintln!("Failed to load userscripts: {}", e);
    }

    let main_window_ = main_window.clone();
    let js_bundle = load_js_bundle();

    unsafe {
        main_window
            .webview
            .AddScriptToExecuteOnDocumentCreated(
                PCWSTR(utils::create_utf_string(js_bundle).as_ptr()),
                &AddScriptToExecuteOnDocumentCreatedCompletedHandler::create(Box::new(move |_, id| {
                    *SCRIPT_ID.lock().unwrap() = id;
                    Ok(())
                })),
            )
            .ok();

        set_handlers(&main_window.webview, &main_window.env);

        main_window
            .webview
            .AddWebResourceRequestedFilter(
                PCWSTR(utils::create_utf_string("*://matchmaker.krunker.io/game-info*").as_ptr()),
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
            )
            .ok();

        if config_bool("realPing", false) {
            modules::ping::load(&main_window.webview);
        }

        let main_window_for_message = main_window.clone();
        let discord_client_for_message = discord_client.clone();
        let mut web_message_token = 0i64;
        main_window
            .webview
            .add_WebMessageReceived(
                &WebMessageReceivedEventHandler::create(Box::new(
                    move |webview, args| {
                        let Some(webview) = webview else {
                            return Ok(());
                        };
                        let Some(args) = args else {
                            return Ok(());
                        };
                        let mut message = PWSTR::null();
                        args.TryGetWebMessageAsString(&mut message).ok();
                        let message_string = take_pwstr(message);
                        handle_web_message(
                            &webview,
                            &main_window_for_message,
                            &discord_client_for_message,
                            &message_string,
                        )
                    },
                )),
                &mut web_message_token,
            )
            .ok();

        main_window.webview.Navigate(w!("https://krunker.io")).ok();

        let mut accelerator_token = 0i64;
        main_window
            .controller
            .clone()
            .add_AcceleratorKeyPressed(
                &AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else {
                        return Ok(());
                    };

                    let mut key_event_kind = COREWEBVIEW2_KEY_EVENT_KIND::default();
                    args.KeyEventKind(&mut key_event_kind)?;
                    if key_event_kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN {
                        return Ok(());
                    }
                    let mut pressed_key: u32 = 0;
                    args.VirtualKey(&mut pressed_key)?;

                    main_window.handle_accelerator_key(pressed_key as u16);
                    Ok(())
                })),
                &mut accelerator_token,
            )
            .ok();
    }

    main_window_
}

fn main() {
    modules::lifecycle::register_instance();
    #[cfg(feature = "packaged")]
    {
        modules::lifecycle::set_panic_hook().ok();
        modules::lifecycle::installer_cleanup().ok();
    }

    if let Err(e) = init_fs() {
        eprintln!("failed to set all the files in place {}", e);
    }

    let window = create_main_window(None);
    let (_tx, rx) = sync::mpsc::channel::<String>();
    #[cfg(feature = "packaged")]
    {
        let main_thread_id = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
        if CONFIG.lock().unwrap().get("checkUpdates").unwrap_or(true) {
            std::thread::spawn(move || {
                modules::lifecycle::check_major_update();
                if let Some(new_js) = modules::lifecycle::check_minor_update() {
                    _tx.send(new_js).ok();
                    unsafe {
                        PostThreadMessageW(main_thread_id, constants::WM_MINOR_UPDATE_READY, WPARAM(0), LPARAM(0))
                            .unwrap();
                    }
                }
            });
        }
    }
    let mut msg: MSG = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            if msg.message == constants::WM_MINOR_UPDATE_READY
                && let Ok(js_content) = rx.try_recv()
            {
                println!("updating js, {}", &*SCRIPT_ID.lock().unwrap());
                window
                    .webview
                    .RemoveScriptToExecuteOnDocumentCreated(PCWSTR(
                        utils::create_utf_string(&*SCRIPT_ID.lock().unwrap()).as_ptr(),
                    ))
                    .ok();
                window
                    .webview
                    .AddScriptToExecuteOnDocumentCreated(PCWSTR(utils::create_utf_string(js_content).as_ptr()), None)
                    .ok();
            }
            DispatchMessageW(&msg);
        }
    }
    // code here runs after window is closed

    CONFIG.lock().unwrap().save();
}
