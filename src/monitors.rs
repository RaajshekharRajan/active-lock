use winit::event_loop::ActiveEventLoop;
use winit::monitor::MonitorHandle;

pub struct MonitorInfo {
    pub handle: MonitorHandle,
    pub is_primary: bool,
}

pub fn enumerate(event_loop: &ActiveEventLoop) -> Vec<MonitorInfo> {
    let primary = event_loop.primary_monitor();
    let monitors: Vec<MonitorHandle> = event_loop.available_monitors().collect();

    if monitors.is_empty() {
        log::warn!("no monitors detected; falling back to primary");
        if let Some(p) = primary {
            return vec![MonitorInfo {
                handle: p,
                is_primary: true,
            }];
        }
        return Vec::new();
    }

    monitors
        .into_iter()
        .enumerate()
        .map(|(i, handle)| {
            let is_primary = primary
                .as_ref()
                .map_or(i == 0, |p| p.name() == handle.name());
            MonitorInfo { handle, is_primary }
        })
        .collect()
}
