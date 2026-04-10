use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use softbuffer::Surface;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::ControlFlow;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::{config, monitors, overlay, platform, ui};

struct OverlayWindow {
    window: Arc<Window>,
    surface: Surface<Arc<Window>, Arc<Window>>,
    is_primary: bool,
    width: u32,
    height: u32,
    scale: f32,
}

pub struct App {
    windows: Vec<OverlayWindow>,
    password_buffer: String,
    error_flash_remaining: u32,
    platform_lock: Option<Box<dyn platform::PlatformLock>>,
    engaged: Arc<AtomicBool>,
    last_focus_check: Instant,
}

impl App {
    pub fn new(engaged: Arc<AtomicBool>) -> Self {
        Self {
            windows: Vec::new(),
            password_buffer: String::new(),
            error_flash_remaining: 0,
            platform_lock: None,
            engaged,
            last_focus_check: Instant::now(),
        }
    }

    fn handle_key(&mut self, event: &KeyEvent, event_loop: &ActiveEventLoop) {
        match &event.logical_key {
            Key::Named(NamedKey::Enter) => {
                if config::verify_password(&self.password_buffer) {
                    self.unlock(event_loop);
                } else {
                    self.error_flash_remaining = config::ERROR_FLASH_FRAMES;
                    self.password_buffer.clear();
                    self.request_primary_redraw();
                }
            }
            Key::Named(NamedKey::Backspace) => {
                self.password_buffer.pop();
                self.request_primary_redraw();
            }
            Key::Named(NamedKey::Escape) => {
                self.password_buffer.clear();
                self.request_primary_redraw();
            }
            _ => {
                if let Some(ref text) = event.text {
                    let s: &str = text;
                    if !s.is_empty() && s.chars().all(|c| !c.is_control()) {
                        self.password_buffer.push_str(s);
                        self.request_primary_redraw();
                    }
                }
            }
        }
    }

    fn unlock(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("correct password — unlocking");
        if let Some(mut lock) = self.platform_lock.take() {
            let _ = lock.disengage();
            // Prevent Drop from running disengage again
            std::mem::forget(lock);
        }
        self.engaged.store(false, Ordering::SeqCst);
        event_loop.exit();
    }

    fn request_primary_redraw(&self) {
        for w in &self.windows {
            if w.is_primary {
                w.window.request_redraw();
            }
        }
    }

    fn render(&mut self, window_id: WindowId) {
        let ow = match self.windows.iter_mut().find(|w| w.window.id() == window_id) {
            Some(w) => w,
            None => return,
        };

        let width = ow.width;
        let height = ow.height;

        if width == 0 || height == 0 {
            return;
        }

        let pixmap = if ow.is_primary {
            let error = self.error_flash_remaining > 0;
            if error {
                self.error_flash_remaining = self.error_flash_remaining.saturating_sub(1);
            }
            ui::render_lock_screen(width, height, ow.scale, self.password_buffer.len(), error)
        } else {
            ui::render_black_screen(width, height)
        };

        let Some(pixmap) = pixmap else { return };
        let data = pixmap.data();

        if let Err(e) = ow.surface.resize(
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        ) {
            log::error!("surface resize failed: {e}");
            return;
        }

        let Ok(mut buffer) = ow.surface.buffer_mut() else {
            return;
        };

        for (i, pixel) in buffer.iter_mut().enumerate() {
            let off = i * 4;
            let r = data[off] as u32;
            let g = data[off + 1] as u32;
            let b = data[off + 2] as u32;
            *pixel = (r << 16) | (g << 8) | b;
        }

        if let Err(e) = buffer.present() {
            log::error!("present failed: {e}");
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.windows.is_empty() {
            return;
        }

        let monitor_list = monitors::enumerate(event_loop);
        log::info!("detected {} monitor(s)", monitor_list.len());

        for info in &monitor_list {
            let attrs = overlay::window_attributes(info);
            let window = match event_loop.create_window(attrs) {
                Ok(w) => Arc::new(w),
                Err(e) => {
                    log::error!("failed to create overlay window: {e}");
                    continue;
                }
            };

            overlay::augment_window(&window);

            let context = match softbuffer::Context::new(window.clone()) {
                Ok(c) => c,
                Err(e) => {
                    log::error!("softbuffer context error: {e}");
                    continue;
                }
            };
            let surface = match Surface::new(&context, window.clone()) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("softbuffer surface error: {e}");
                    continue;
                }
            };

            let size = window.inner_size();
            let scale = window.scale_factor() as f32;

            self.windows.push(OverlayWindow {
                window,
                surface,
                is_primary: info.is_primary,
                width: size.width,
                height: size.height,
                scale,
            });
        }

        let mut lock = platform::create_platform_lock();
        if let Err(e) = lock.engage() {
            log::error!("failed to engage platform lock: {e}");
        }
        self.engaged.store(true, Ordering::SeqCst);
        self.platform_lock = Some(lock);

        for w in &self.windows {
            w.window.request_redraw();
            w.window.focus_window();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => { /* blocked */ }

            WindowEvent::Focused(false) => {
                if let Some(w) = self.windows.iter().find(|w| w.window.id() == window_id) {
                    w.window.focus_window();
                }
            }

            WindowEvent::KeyboardInput {
                event: key_event, ..
            } if key_event.state == ElementState::Pressed => {
                self.handle_key(&key_event, event_loop);
            }

            WindowEvent::RedrawRequested => {
                self.render(window_id);
            }

            WindowEvent::Resized(new_size) => {
                if let Some(w) = self.windows.iter_mut().find(|w| w.window.id() == window_id) {
                    w.width = new_size.width;
                    w.height = new_size.height;
                    w.window.request_redraw();
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();

        if self.error_flash_remaining > 0 {
            event_loop.set_control_flow(ControlFlow::WaitUntil(now + Duration::from_millis(16)));
            self.request_primary_redraw();
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                now + Duration::from_millis(config::FOCUS_POLL_MS),
            ));
        }

        if now.duration_since(self.last_focus_check) >= Duration::from_millis(config::FOCUS_POLL_MS)
        {
            self.last_focus_check = now;
            for w in &self.windows {
                w.window.focus_window();
            }
        }
    }
}
