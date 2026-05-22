use crate::modules::fps_stats::{self, FpsStats};

use std::sync::{Arc, Mutex};

pub fn spawn_imgui_window(fps_stats: Arc<Mutex<FpsStats>>) {
    std::thread::spawn(move || {
        use glium::glutin::event::{Event, WindowEvent};
        use glium::glutin::event_loop::EventLoop;
        use glium::glutin::platform::windows::EventLoopExtWindows;
        use glium::glutin::window::WindowBuilder;
        use glium::{Display, Surface};
        use imgui_winit_support::WinitPlatform;

        let event_loop: EventLoop<()> = EventLoop::new_any_thread();

        let window = WindowBuilder::new()
            .with_title("glorp | FPS Monitor")
            .with_inner_size(glium::glutin::dpi::LogicalSize::new(400.0, 200.0));

        let cb = glium::glutin::ContextBuilder::new();
        let display = Display::new(window, cb, &event_loop).unwrap();

        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        // --- NEW: wire up the winit platform ---
        let mut platform = WinitPlatform::init(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            display.gl_window().window(),
            imgui_winit_support::HiDpiMode::Default,
        );

        imgui.fonts().add_font(&[imgui::FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                size_pixels: 13.0,
                ..Default::default()
            }),
        }]);

        let mut renderer = imgui_glium_renderer::Renderer::init(&mut imgui, &display).unwrap();
        let mut last_frame = std::time::Instant::now();

        event_loop.run(move |event, _, control_flow| {
            let now = std::time::Instant::now();
            let delta = now - last_frame;
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

                // --- FPS window ---
                let stats = fps_stats.lock().unwrap().clone();

                imgui::Window::new("FPS Monitor")
                    .size([400.0, 220.0], imgui::Condition::FirstUseEver)
                    .build(&ui, || {
                        // Color the FPS value: green > 60, yellow > 30, red otherwise
                        let color = if stats.current >= 60.0 {
                            [0.0, 1.0, 0.3, 1.0]
                        } else if stats.current >= 30.0 {
                            [1.0, 0.8, 0.0, 1.0]
                        } else {
                            [1.0, 0.2, 0.2, 1.0]
                        };

                        let _c = ui.push_style_color(imgui::StyleColor::Text, color);
                        ui.text(format!("FPS:     {:>5}", stats.current));
                        drop(_c);

                        ui.text(format!("Avg:     {:>5}", stats.average));
                        ui.text(format!("1% Low:  {:>5}", stats.low1));
                        ui.text(format!("0.1% Low:{:>5}", stats.low01));

                        ui.separator();

                        // Plot the history as a scrolling graph
                        if !stats.history.is_empty() {
                            let overlay = format!("{} fps", stats.current);
                            let (front_slice, _) = stats.history.as_slices();
                            ui.plot_lines("##fps_plot", front_slice)
                                .overlay_text(&overlay)
                                .scale_min(0.0)
                                .scale_max(1000.0) // upper bound for Krunker FPS
                                .graph_size([380.0, 80.0])
                                .build();
                        }
                    });

                platform.prepare_render(&ui, display.gl_window().window());
                let mut target = display.draw();
                target.clear_color_srgb(0.1, 0.1, 0.1, 1.0);
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