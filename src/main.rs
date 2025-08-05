use log::info;

mod sip_client;
mod components;
mod event_channel;

use components::App;

fn main() {
    // Initialize logging
    env_logger::init();
    
    info!("Starting SIP Client");
    
    // Launch the Dioxus desktop application
    dioxus::launch(App);
} 