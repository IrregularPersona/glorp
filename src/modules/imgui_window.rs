use std::collections::VecDeque;
use glium::glutin::platform::windows::WindowExtWindows;
use windows::Win32::Foundation::{COLORREF, HWND};
use crate::utils::debug_print;

struct OverlaySettings {
    show_fps:       bool,
    show_frametime: bool,
    show_avg:       bool,
    show_low1:      bool,
    show_low01:     bool,
    show_graph:     bool,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            show_fps:       true,
            show_frametime: true,
            show_avg:       true,
            show_low1:      true,
            show_low01:     true,
            show_graph:     true,
        }
    }
}

pub fn spawn_imgui_window(parent_hwnd: HWND) {

    std::panic::set_hook(Box::new(|info| {
        debug_print(format!("[glorp] PANIC: {}", info));
    }));


    // it deadass has to be here cuz u cant move c_void pointers between threads safely LMFAO
    let parent = UnsafeSend::new(parent_hwnd);

    std::thread::spawn(move || {
        use glium::glutin::event::{Event, VirtualKeyCode, WindowEvent};
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
        
        debug_print(format!("[glorp] gl vendor: {:?}", display.get_opengl_vendor_string()));
        debug_print(format!("[glorp] gl renderer: {:?}", display.get_opengl_renderer_string()));

        let overlay_hwnd = HWND(display.gl_window().window().hwnd() as *mut _);

        unsafe {
            let style = GetWindowLongPtrW(overlay_hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(
                overlay_hwnd,
                GWL_EXSTYLE,
                (style
                    | WS_EX_TRANSPARENT.0 as isize
                    | WS_EX_LAYERED.0 as isize
                    | WS_EX_TOPMOST.0 as isize
                    | WS_EX_TOOLWINDOW.0 as isize)
                    & !(WS_EX_APPWINDOW.0 as isize),
            );
            SetLayeredWindowAttributes(overlay_hwnd, COLORREF(0), 255, LWA_ALPHA).ok();
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

        // let font_data = std::fs::read("C:\\Windows\\Fonts\\segoeui.ttf")
        //     .expect("Failed to load system font");
        debug_print("[glorp] font file read ok");

        let mut imgui = imgui::Context::create();
        debug_print("[glorp] imgui context created");
        imgui.set_ini_filename(None);
        imgui::Style::use_light_colors(imgui.style_mut()); 
        // imgui.style_mut().colors[imgui::StyleColor::Text as usize] =
        //     [1.0, 1.0, 1.0, 1.0];


        // let _font_data_guard = &font_data;

        // imgui.fonts().add_font(&[
        //     imgui::FontSource::TtfData {
        //         data: &font_data,
        //         size_pixels: 20.0,
        //         config: None,
        //     }
        // ]);

        imgui.fonts().add_font(&[imgui::FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                size_pixels: 24.0,
                ..Default::default()
            }),
        }]);

        {
            let mut fonts = imgui.fonts();
            let tex = fonts.build_rgba32_texture();
            debug_print(format!("[glorp] atlas: {}x{} len={}", tex.width, tex.height, tex.data.len()));
        }



        let mut platform = WinitPlatform::init(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            display.gl_window().window(),
            imgui_winit_support::HiDpiMode::Default,
        );

        let mut renderer = imgui_glium_renderer::Renderer::init(&mut imgui, &display).unwrap();
        debug_print("[glorp] renderer init ok");
        let mut last_frame = std::time::Instant::now();
        let mut last_read = std::time::Instant::now();
        let mut ft_history: VecDeque<f32> = VecDeque::with_capacity(100);
        let mut current_fps = 0.0f32;
        let mut current_ft = 0.0f32;

        let mut settings = OverlaySettings::default();
        let mut show_settings = false;

        event_loop.run(move |event, _, control_flow| {
            let now = std::time::Instant::now();
            let _delta = now - last_frame;
            last_frame = now;

            match &event {
                Event::WindowEvent { event: WindowEvent::KeyboardInput { input, .. }, .. } => {
                    // toggle settings window on Page Up key idk
                    if input.state == glium::glutin::event::ElementState::Pressed {
                        if let Some(VirtualKeyCode::PageUp) = input.virtual_keycode {
                            show_settings = !show_settings;
                        }
                    }
                }
                _ => {}
            }

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = glium::glutin::event_loop::ControlFlow::Exit;
                }

                Event::MainEventsCleared => {
                    thread_local! {
                        static FRAME_COUNT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
                    }
                    FRAME_COUNT.with(|c| {
                        let n = c.get();
                        if n < 5 {
                            debug_print(format!("[glorp] frame {n}"));
                        }
                        c.set(n + 1);
                    });

                    // debug_print(format!("[glorp] render ok frame {}", FRAME_COUNT.with(|c| c.get())));


                    if last_read.elapsed() >= std::time::Duration::from_millis(250) {
                        if let Some((fps, ft)) = read_frame_data() {
                            current_fps = fps;
                            current_ft = ft;
                            ft_history.push_back(ft);
                            if ft_history.len() > 100 {
                                ft_history.pop_front();
                            }
                        }
                        last_read = std::time::Instant::now();
                    }

                    platform
                        .prepare_frame(imgui.io_mut(), display.gl_window().window())
                        .expect("Failed to prepare frame");

                    let ui = imgui.frame();

                    let (average, low1, low01) = calculate_stats(&ft_history);

                    // --- overlay ---
                    let _no_bg     = ui.push_style_color(imgui::StyleColor::WindowBg, [30.0, 0.0, 0.0, 0.0]);
                    let _no_border = ui.push_style_color(imgui::StyleColor::Border,   [0.0, 0.0, 0.0, 0.0]);
                    let _text      = ui.push_style_color(imgui::StyleColor::Text,      [1.0, 1.0, 1.0, 1.0]);

                    imgui::Window::new("##overlay")
                        .size([300.0, 300.0], imgui::Condition::Always)
                        .position([20.0, 20.0], imgui::Condition::Always)
                        .flags(
                            imgui::WindowFlags::NO_TITLE_BAR
                            | imgui::WindowFlags::NO_RESIZE
                            | imgui::WindowFlags::NO_MOVE
                            | imgui::WindowFlags::NO_SCROLLBAR
                            | imgui::WindowFlags::NO_INPUTS
                            | imgui::WindowFlags::NO_SAVED_SETTINGS,
                        )
                        .build(&ui, || {
                            if settings.show_fps {
                                let color = if current_fps >= 60.0 {
                                    [0.0, 1.0, 0.3, 1.0]
                                } else if current_fps >= 30.0 {
                                    [1.0, 0.8, 0.0, 1.0]
                                } else {
                                    [1.0, 0.2, 0.2, 1.0]
                                };
                                let _c = ui.push_style_color(imgui::StyleColor::Text, color);
                                ui.text(format!("FPS:      {:>6.1}", current_fps));
                            }

                            if settings.show_frametime {
                                ui.text(format!("FT:       {:>5.2} ms", current_ft));
                            }

                            if settings.show_avg {
                                ui.text(format!("Avg FT:   {:>5.2} ms", average));
                            }

                            if settings.show_low1 {
                                ui.text(format!("99th%%:   {:>5.2} ms", low1));
                            }

                            if settings.show_low01 {
                                ui.text(format!("99.9th%%: {:>5.2} ms", low01));
                            }
                        });

                    drop(_no_bg);
                    drop(_no_border);
                    drop(_text);

                    // --- settings window ---
                    if show_settings {
                        imgui::Window::new("glorp | settings")
                            .size([280.0, 240.0], imgui::Condition::Always)
                            .position([320.0, 150.0], imgui::Condition::FirstUseEver)
                            .flags(
                                imgui::WindowFlags::NO_RESIZE
                                | imgui::WindowFlags::NO_SAVED_SETTINGS,
                            )
                            .opened(&mut show_settings)
                            .build(&ui, || {
                                ui.text("Overlay metrics");
                                ui.separator();
                                ui.checkbox("FPS",               &mut settings.show_fps);
                                ui.checkbox("Frame time (ms)",   &mut settings.show_frametime);
                                ui.checkbox("Avg frame time",    &mut settings.show_avg);
                                ui.checkbox("99th percentile",   &mut settings.show_low1);
                                ui.checkbox("99.9th percentile", &mut settings.show_low01);
                                ui.checkbox("Graph",             &mut settings.show_graph);
                            });
                    }

                    platform.prepare_render(&ui, display.gl_window().window());
                    let mut target = display.draw();
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

    // 99th and 99.9th percentile frametimes — higher = worse
    let p99 = sorted[((sorted.len() as f32 * 0.99) as usize).min(sorted.len() - 1)];
    let p999 = sorted[((sorted.len() as f32 * 0.999) as usize).min(sorted.len() - 1)];

    (average, p99, p999)
}

use windows::{Win32::System::Memory::*, core::s};
use crate::utils::UnsafeSend;

pub fn read_frame_data() -> Option<(f32, f32)> {
    unsafe {
        let mapping = OpenFileMappingA(FILE_MAP_READ.0, false, s!("GlorpFrameTiming")).ok()?;
        let ptr = MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, 64);
        if ptr.Value.is_null() { return None; }
        let frame_ns = *(ptr.Value as *const u64);
        let frame_ms = frame_ns as f32 / 1_000_000.0;
        let fps = 1000.0 / frame_ms.max(0.001);
        Some((fps, frame_ms))
    }
}