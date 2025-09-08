// SPDX-License-Identifier: GPL-3.0-only

use cosmic::applet::cosmic_panel_config::{PanelSize, PanelAnchor};
use cosmic::applet::{PanelType, Size};
use cosmic::app::{Core, Task};
use cosmic::iced::futures::channel;
use cosmic::iced::{Limits, Subscription, window::Id,};
use cosmic::iced::Limits;
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::widget::{self, settings, vertical_space, slider, list_column};

use cosmic::{Application, Element};

use tokio_udev::Device;

use crate::fl;

#[derive(Default)]
pub struct LaunchControl {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// The popup id.
    popup: Option<Id>,
    /// Example row toggler.
    example_row: bool,
}

pub struct DeviceInfo {
    vid: u32,
    pid: u32
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    ToggleExampleRow(bool),
    DeviceConnected(DeviceInfo),
    DeviceDisconnected
}

impl LaunchControl {
    async fn device_task(mut out: Subscription<Self::Message>) {
        if let Ok(mut rx) = DeviceListener::new(0x3384, 0x0001..=0x000A)
            .with_subsystem("hidraw")
            .with_debounce_ms(300)
            .start()
            .await
        {
            while let Some(ev) = rx.recv().await {
                // forward events into iced
                let _ = out.send(Message::Device(ev)).await;
            }
        }
    }
}


impl Application for LaunchControl {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.erinxocon.CosmicAppletLaunchControl";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        channel("device-listener", 128, LaunchControl::device_task)
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let app = LaunchControl {
            core,
            ..Default::default()
        };

        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }


    fn view(&self) -> Element<'_, Self::Message> {
        self.core
            .applet
            .icon_button("display-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let content_list = list_column()
            .padding(5)
            .spacing(0)
            .add(settings::item(
                fl!("example-row"),
                widget::toggler(self.example_row).on_toggle(Message::ToggleExampleRow),
            ));

        self.core.applet.popup_container(content_list).into()
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(372.0)
                        .min_width(300.0)
                        .min_height(200.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
                }
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::ToggleExampleRow(toggled) => self.example_row = toggled,
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}
