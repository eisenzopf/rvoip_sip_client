use log::info;

mod sip_client;
mod components;
mod event_channel;
mod network_utils;
mod commands;

use components::App;

fn main() {
    // Initialize logging
    env_logger::init();
    
    info!("Starting SIP Client");
    
    // Launch the Dioxus desktop application with custom window title and size
    dioxus::LaunchBuilder::desktop()
        .with_cfg(dioxus::desktop::Config::new()
            .with_window(dioxus::desktop::WindowBuilder::new()
                .with_title("RVOIP SIP Client")
                .with_inner_size(dioxus::desktop::LogicalSize::new(600.0, 800.0))))
        .launch(App);
} 