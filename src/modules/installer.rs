#![allow(dead_code)]
use std::io::Read;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::{Win32::UI::WindowsAndMessaging::*, core::*};
const INSTALLER_URL: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";
const INSTALLER_FILENAME: &str = "MicrosoftEdgeWebView2Setup.exe";

const UPDATE_URL: &str = "https://api.github.com/repos/slavcp/glorp/releases/latest";

pub fn check_webview2() {
    unsafe {
        let mut version_string: Vec<u16> = crate::utils::create_utf_string("");
        let version_info = version_string.as_mut_ptr() as *mut PWSTR;
        match GetAvailableCoreWebView2BrowserVersionString(None, version_info) {
            Ok(_) => {}
            Err(e) => {
                println!("Error getting webview version: {:?}", e);
                match MessageBoxW(
                    None,
                    w!("Client cannot launch with missing Webview2 runtime, install?"),
                    w!("Error launching client"),
                    MB_ICONERROR | MB_YESNO | MB_SYSTEMMODAL,
                ) {
                    IDYES => install_webview2(),
                    _ => std::process::exit(0),
                }
            }
        }
    }
}

fn install_webview2() {
    unsafe {
        let mut output_path = std::env::current_exe().unwrap();
        output_path.pop();
        output_path.push(INSTALLER_FILENAME);
        println!("output path: {}", output_path.display());
        let response = match ureq::get(INSTALLER_URL).call() {
            Ok(response) => response,
            Err(e) => {
                MessageBoxW(
                    None,
                    PCWSTR(
                        crate::utils::create_utf_string(
                            format!("Cannot download runtime: {}", e).as_str(),
                        )
                        .as_ptr(),
                    ),
                    w!("Error downloading"),
                    MB_ICONERROR | MB_SYSTEMMODAL,
                );
                std::process::exit(0);
            }
        };
        println!("Downloading to {}", output_path.display());
        {
            let mut out = std::fs::File::options()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&output_path)
                .unwrap();
            if let Err(e) = std::io::copy(&mut response.into_body().as_reader(), &mut out) {
                MessageBoxW(
                    None,
                    PCWSTR(
                        crate::utils::create_utf_string(
                            format!("failed to copy installer file: {}", e).as_str(),
                        )
                        .as_ptr(),
                    ),
                    w!("Error installing"),
                    MB_ICONERROR | MB_SYSTEMMODAL,
                );
                std::process::exit(0);
            };
        }
        let mut command = std::process::Command::new(&output_path);

        // blocks thread until the installer finishes
        match command.status() {
            Ok(status) => status,
            Err(e) => {
                eprintln!("Error spawning installer process: {}", e);
                return;
            }
        };

        std::fs::remove_file(&output_path).ok();
    }
}
pub fn check_update() {
    std::thread::spawn(|| {
        let mut response = match ureq::get(UPDATE_URL).call() {
            Ok(response) => response.into_body(),
            Err(_) => return,
        };
        let mut buf = String::new();
        response.as_reader().read_to_string(&mut buf).unwrap();

        let json = serde_json::from_str::<serde_json::Value>(&buf).unwrap();
        let newest_version = match json["tag_name"].as_str() {
            Some(v) => v,
            None => return,
        };
        if semver::Version::parse(newest_version).unwrap()
            <= semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap()
        {
            return;
        };

        let download_url = match json["assets"][0]["browser_download_url"].as_str() {
            Some(url) => url,
            None => return,
        };

        let mut output_path = std::env::current_exe().unwrap();
        output_path.pop();
        output_path.push(format!("version.{}.msi", newest_version));

        let res = match ureq::get(download_url).call() {
            Ok(res) => res,
            Err(e) => {
                unsafe {
                    MessageBoxW(
                        None,
                        PCWSTR(
                            crate::utils::create_utf_string(
                                format!("Failed to download: {:?}", e).as_str(),
                            )
                            .as_ptr(),
                        ),
                        w!("Download Error"),
                        MB_ICONERROR | MB_SYSTEMMODAL,
                    );
                }
                return;
            }
        };

        let mut file = match std::fs::File::create(&output_path) {
            Ok(file) => file,
            Err(e) => {
                unsafe {
                    MessageBoxW(
                        None,
                        PCWSTR(
                            crate::utils::create_utf_string(
                                format!("Failed to create file: {:?}", e).as_str(),
                            )
                            .as_ptr(),
                        ),
                        w!("Download Error"),
                        MB_ICONERROR | MB_SYSTEMMODAL,
                    );
                }
                return;
            }
        };

        if let Err(e) = std::io::copy(&mut res.into_body().as_reader(), &mut file) {
            panic!("Failed to download: {:?}", e)
        }
        drop(file);
        unsafe {
            if let MESSAGEBOX_RESULT(6) = MessageBoxW(
                None,
                w!("A new version is available, update?"),
                w!("Update available"),
                MB_ICONQUESTION | MB_YESNO,
            ) {
                ShellExecuteW(
                    None,
                    w!("open"),
                    PCWSTR(crate::utils::create_utf_string(output_path.to_str().unwrap()).as_ptr()),
                    w!("/q"),
                    None,
                    SW_NORMAL,
                );
                std::process::exit(0);
            }
        }
    });
}
