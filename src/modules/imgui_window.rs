use std::collections::VecDeque;
use windows::Win32::Foundation::HWND;

struct SendHwnd(HWND);
unsafe impl Send for SendHwnd {}

pub fn spawn_imgui_window(parent_hwnd: HWND) {

    // it deadass has to be here cuz u cant move c_void pointers between threads safely LMFAO
    let parent = UnsafeSend::new(parent_hwnd);

    std::thread::spawn(move || {
        use glium::glutin::event::{Event, WindowEvent};
        use glium::glutin::event_loop::EventLoop;
        use glium::glutin::platform::windows::EventLoopExtWindows;
        use glium::glutin::window::WindowBuilder;
        use glium::{Display, Surface};
        use imgui_winit_support::WinitPlatform;
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::Win32::Foundation::RECT;

        let event_loop: EventLoop<()> = EventLoop::new_any_thread();

        let window = WindowBuilder::new()
            .with_title("glorp | overlay")
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top(true)
            .with_inner_size(glium::glutin::dpi::LogicalSize::new(1.0, 1.0));

        let cb = glium::glutin::ContextBuilder::new();
        let display = Display::new(window, cb, &event_loop).unwrap();

        
        // get overlay HWND and make it layered + click-through
        let overlay_hwnd = unsafe {
            let title: Vec<u16> = "glorp | overlay\0".encode_utf16().collect();
            FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())).unwrap()
        };

        unsafe {
            let style = GetWindowLongPtrW(overlay_hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(overlay_hwnd, GWL_EXSTYLE, style | WS_EX_TRANSPARENT.0 as isize | WS_EX_LAYERED.0 as isize);
        }

        let overlay = UnsafeSend::new(overlay_hwnd);

        std::thread::spawn(move || unsafe {
            let parent_hwnd = *parent;
            let overlay_hwnd = *overlay;
            loop {
                let mut rect = RECT::default();
                if GetWindowRect(parent_hwnd, &mut rect).is_ok() {
                    SetWindowPos(
                        overlay_hwnd,
                        Some(HWND_TOPMOST),
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                        SWP_NOACTIVATE,
                    ).ok();
                }
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
        });

        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        let mut platform = WinitPlatform::init(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            display.gl_window().window(),
            imgui_winit_support::HiDpiMode::Default,
        );

        imgui.fonts().add_font(&[imgui::FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                size_pixels:32.0,
                ..Default::default()
            }),
        }]);

        let mut renderer = imgui_glium_renderer::Renderer::init(&mut imgui, &display).unwrap();
        let mut last_frame = std::time::Instant::now();
        let mut fps_history: VecDeque<f32> = VecDeque::with_capacity(100);
        let mut current_fps = 0.0f32;

        event_loop.run(move |event, _, control_flow| {
            let now = std::time::Instant::now();
            let _delta = now - last_frame;
            last_frame = now;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = glium::glutin::event_loop::ControlFlow::Exit;
                }

                Event::MainEventsCleared => {
                    platform
                        .prepare_frame(imgui.io_mut(), display.gl_window().window())
                        .expect("Failed to prepare frame");

                    let ui = imgui.frame();

                    if let Some(fps) = read_gpu_frame_time() {
                        current_fps = fps;
                        fps_history.push_back(fps);
                        if fps_history.len() > 100 {
                            fps_history.pop_front();
                        }
                    }

                    let (average, low1, low01) = calculate_stats(&fps_history);

                    // no background, no border, no titlebar
                    let _no_bg = ui.push_style_color(imgui::StyleColor::WindowBg, [0.0, 0.0, 0.0, 0.0]);
                    let _no_border = ui.push_style_color(imgui::StyleColor::Border, [0.0, 0.0, 0.0, 0.0]);

                    imgui::Window::new("##overlay")
                        .size([300.0, 200.0], imgui::Condition::Always)
                        .position([10.0, 150.0], imgui::Condition::Always)
                        .flags(
                            imgui::WindowFlags::NO_TITLE_BAR
                            | imgui::WindowFlags::NO_RESIZE
                            | imgui::WindowFlags::NO_MOVE
                            | imgui::WindowFlags::NO_SCROLLBAR
                            | imgui::WindowFlags::NO_INPUTS
                            | imgui::WindowFlags::NO_SAVED_SETTINGS,
                        )
                        .build(&ui, || {
                            let color = if current_fps >= 60.0 {
                                [0.0, 1.0, 0.3, 1.0]
                            } else if current_fps >= 30.0 {
                                [1.0, 0.8, 0.0, 1.0]
                            } else {
                                [1.0, 0.2, 0.2, 1.0]
                            };

                            let _c = ui.push_style_color(imgui::StyleColor::Text, color);
                            ui.text(format!("FPS:     {:>5.1}", current_fps));
                            drop(_c);

                            ui.text(format!("Avg:     {:>5.1}", average));
                            ui.text(format!("1% Low:  {:>5.1}", low1));
                            ui.text(format!("0.1% Low:{:>5.1}", low01));

                            if !fps_history.is_empty() {
                                let overlay = format!("{:.1} fps", current_fps);
                                let (front_slice, _) = fps_history.as_slices();
                                ui.plot_lines("##fps_plot", front_slice)
                                    .overlay_text(&overlay)
                                    .scale_min(0.0)
                                    .scale_max(2000.0)
                                    .graph_size([240.0, 80.0])
                                    .build();
                            }
                        });

                    drop(_no_bg);
                    drop(_no_border);

                    platform.prepare_render(&ui, display.gl_window().window());
                    let mut target = display.draw();
                    // fully transparent clear
                    target.clear_color_srgb(0.0, 0.0, 0.0, 0.0);
                    let draw_data = ui.render();
                    renderer.render(&mut target, draw_data).expect("Render failed");
                    target.finish().ok();
                }

                event => {
                    platform.handle_event(
                        imgui.io_mut(),
                        display.gl_window().window(),
                        &event,
                    );
                }
            }
        });
    });
}

fn calculate_stats(history: &VecDeque<f32>) -> (f32, f32, f32) {
    if history.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let mut sorted = history.iter().copied().collect::<Vec<_>>();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let average = sorted.iter().sum::<f32>() / sorted.len() as f32;

    // Calculate 1st and 0.1st percentiles (low fps)
    let low1 = if sorted.len() > 1 {
        let idx = ((sorted.len() as f32 * 0.01).ceil() as usize).saturating_sub(1);
        sorted[idx]
    } else {
        sorted[0]
    };

    let low01 = if sorted.len() > 1 {
        let idx = ((sorted.len() as f32 * 0.001).ceil() as usize).saturating_sub(1);
        sorted[idx]
    } else {
        sorted[0]
    };

    (average, low1, low01)
}

use windows::core::PCWSTR;
use windows::{Win32::System::Memory::*, core::s};

use crate::utils::UnsafeSend;

pub fn read_gpu_frame_time() -> Option<f32> {
    unsafe {
        let mapping = OpenFileMappingA(
            FILE_MAP_READ.0,
            false,
            s!("GlorpFrameTiming"),
        ).ok()?;
        let ptr = MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, 64);
        if ptr.Value.is_null() { return None; }
        let frame_ns = *(ptr.Value as *const u64);
        let frame_ms = frame_ns as f32 / 1_000_000.0;
        let fps = 1000.0 / frame_ms.max(0.001);
        Some(fps)
    }
}