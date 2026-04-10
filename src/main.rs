#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod config;
mod monitors;
mod overlay;
mod platform;
mod ui;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    env_logger::init();

    if std::env::args().any(|a| a == "--set-password") {
        let password =
            rpassword::prompt_password("Enter new lock password: ").expect("failed to read input");
        if password.is_empty() {
            eprintln!("Password cannot be empty.");
            std::process::exit(1);
        }
        let confirm = rpassword::prompt_password("Confirm password: ").expect("failed to read input");
        if password != confirm {
            eprintln!("Passwords do not match.");
            std::process::exit(1);
        }
        match config::set_password(&password) {
            Ok(()) => eprintln!("Password saved to ~/.active-lock/password.hash"),
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    if std::env::args().any(|a| a == "--reset") {
        eprintln!("Resetting system settings...");
        let mut lock = platform::create_platform_lock();
        match lock.disengage() {
            Ok(()) => eprintln!("System settings restored."),
            Err(e) => eprintln!("Reset error: {e}"),
        }
        return;
    }

    let engaged = Arc::new(AtomicBool::new(false));

    let default_hook = std::panic::take_hook();
    let engaged_panic = engaged.clone();
    std::panic::set_hook(Box::new(move |info| {
        if engaged_panic.load(Ordering::SeqCst) {
            let mut lock = platform::create_platform_lock();
            let _ = lock.disengage();
        }
        default_hook(info);
    }));

    let engaged_ctrlc = engaged.clone();
    ctrlc::set_handler(move || {
        if engaged_ctrlc.load(Ordering::SeqCst) {
            let mut lock = platform::create_platform_lock();
            let _ = lock.disengage();
        }
        std::process::exit(0);
    })
    .expect("failed to install Ctrl-C handler");

    let event_loop = winit::event_loop::EventLoop::new().expect("failed to create event loop");
    let mut application = app::App::new(engaged);
    event_loop
        .run_app(&mut application)
        .expect("event loop error");
}
