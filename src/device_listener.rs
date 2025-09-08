use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tokio_udev::{Enumerator, MonitorBuilder};
use futures::StreamExt;

pub struct DeviceListener {
    subsystem: &'static str,
    debounce: Duration,
}

impl DeviceListener {
    pub fn new() -> Self {
        Self {
            subsystem: "usb",
            debounce: Duration::from_millis(300),
        }
    }

    // Start listening, send Messages into the given sender.
    pub async fn run(self, mut out: iced::subscription::Channel<Message>) {
        let mut last_event: HashMap<String, Instant> = HashMap::new();

        // Enumerate existing devices
        if let Ok(mut enumr) = Enumerator::new() {
            if enumr.match_subsystem(self.subsystem).is_ok() {
                if let Ok(devs) = enumr.scan_devices() {
                    for dev in devs {
                        if let Some(info) = extract_info(&dev) {
                            if should_fire(&mut last_event, &info, self.debounce) {
                                let _ = out.send(Message::DeviceConnected(info)).await;
                            }
                        }
                    }
                }
            }
        }

        // Monitor hotplug events
        let monitor = match MonitorBuilder::new()
            .and_then(|m| m.match_subsystem(self.subsystem))
            .and_then(|m| m.listen())
        {
            Ok(m) => m,
            Err(err) => {
                eprintln!("udev monitor error: {err}");
                return;
            }
        };
        tokio::pin!(monitor);

        while let Some(evt) = monitor.next().await {
            if let Some(info) = extract_info(&evt.device()) {
                if !should_fire(&mut last_event, &info, self.debounce) {
                    continue;
                }
                match evt.event_type().as_str() {
                    "add" => {
                        let _ = out.send(Message::DeviceConnected(info)).await;
                    }
                    "remove" => {
                        let _ = out.send(Message::DeviceDisconnected).await;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn extract_info(dev: &tokio_udev::Device) -> Option<DeviceInfo> {
    let vid = dev
        .property_value("ID_VENDOR_ID")?
        .to_string_lossy()
        .to_string();
    let pid = dev
        .property_value("ID_MODEL_ID")?
        .to_string_lossy()
        .to_string();

    Some(DeviceInfo {
        vid: u32::from_str_radix(&vid, 16).ok()?,
        pid: u32::from_str_radix(&pid, 16).ok()?,
    })
}

fn should_fire(
    last: &mut HashMap<String, Instant>,
    info: &DeviceInfo,
    win: Duration,
) -> bool {
    let key = format!("{:04x}:{:04x}", info.vid, info.pid);
    let now = Instant::now();
    match last.get(&key) {
        Some(prev) if now.duration_since(*prev) < win => false,
        _ => {
            last.insert(key, now);
            true
        }
    }
}
