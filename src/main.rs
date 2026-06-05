use log::info;

mod sip_client;
mod components;
mod event_channel;
mod network_utils;
mod commands;
mod audio;

use components::App;

fn main() {
    // Initialize logging. Default to a quiet filter so rvoip's per-packet DEBUG
    // firehose doesn't starve the real-time audio threads. RUST_LOG overrides
    // this (e.g. `RUST_LOG=debug` for diagnostics).
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,sip_client=info"),
    )
    .init();
    
    info!("Starting SIP Client");
    
    // Launch the Dioxus desktop application with custom window title and size
    dioxus::LaunchBuilder::desktop()
        .with_cfg(dioxus::desktop::Config::new()
            .with_window(dioxus::desktop::WindowBuilder::new()
                .with_title("RVOIP SIP Client")
                .with_inner_size(dioxus::desktop::LogicalSize::new(600.0, 800.0))))
        .launch(App);
} 