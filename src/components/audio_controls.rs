use dioxus::prelude::*;
use log::{info, error};
use std::sync::Arc;

use crate::sip_client::SipClientManager;

#[component]
pub fn AudioControls(
    sip_client: Signal<Arc<tokio::sync::RwLock<SipClientManager>>>,
    call_id: Option<String>,
) -> Element {
    let mut microphone_muted = use_signal(|| false);
    let mut speaker_muted = use_signal(|| false);
    let mut input_volume = use_signal(|| 1.0f32);
    let mut output_volume = use_signal(|| 1.0f32);
    let mut audio_enabled = use_signal(|| true);
    let mut echo_cancellation = use_signal(|| true);
    let mut noise_suppression = use_signal(|| true);
    let mut auto_gain_control = use_signal(|| true);
    let mut audio_active = use_signal(|| false);
    
    // Load initial audio state
    use_effect(move || {
        let client = sip_client.clone();
        spawn(async move {
            let binding = client.peek();
            let client_guard = binding.read().await;
            
            microphone_muted.set(client_guard.is_microphone_muted().await);
            speaker_muted.set(client_guard.is_speaker_muted().await);
            audio_enabled.set(client_guard.is_audio_enabled().await);
            audio_active.set(client_guard.is_audio_active().await);
            
            if let Some(audio_controls) = client_guard.get_audio_controls() {
                input_volume.set(audio_controls.get_input_volume().await);
                output_volume.set(audio_controls.get_output_volume().await);
                echo_cancellation.set(audio_controls.is_echo_cancellation_enabled().await);
                noise_suppression.set(audio_controls.is_noise_suppression_enabled().await);
                auto_gain_control.set(audio_controls.is_auto_gain_control_enabled().await);
            }
        });
    });

    // Toggle microphone mute
    let toggle_microphone_mute = move |_| {
        let client = sip_client.clone();
        spawn(async move {
            let binding = client.peek();
            let client_guard = binding.read().await;
            
            match client_guard.toggle_microphone_mute().await {
                Ok(muted) => {
                    info!("🔇 Microphone mute toggled: {}", muted);
                    microphone_muted.set(muted);
                }
                Err(e) => {
                    error!("Failed to toggle microphone mute: {}", e);
                }
            }
        });
    };

    // Toggle speaker mute
    let toggle_speaker_mute = move |_| {
        let client = sip_client.clone();
        spawn(async move {
            let binding = client.peek();
            let client_guard = binding.read().await;
            
            match client_guard.toggle_speaker_mute().await {
                Ok(muted) => {
                    info!("🔇 Speaker mute toggled: {}", muted);
                    speaker_muted.set(muted);
                }
                Err(e) => {
                    error!("Failed to toggle speaker mute: {}", e);
                }
            }
        });
    };

    // Set input volume
    let set_input_volume = move |evt: Event<FormData>| {
        let volume = evt.value().parse::<f32>().unwrap_or(1.0) / 100.0;
        let client = sip_client.clone();
        input_volume.set(volume);
        
        spawn(async move {
            let binding = client.peek();
            let client_guard = binding.read().await;
            
            if let Err(e) = client_guard.set_input_volume(volume).await {
                error!("Failed to set input volume: {}", e);
            }
        });
    };

    // Set output volume
    let set_output_volume = move |evt: Event<FormData>| {
        let volume = evt.value().parse::<f32>().unwrap_or(1.0) / 100.0;
        let client = sip_client.clone();
        output_volume.set(volume);
        
        spawn(async move {
            let binding = client.peek();
            let client_guard = binding.read().await;
            
            if let Err(e) = client_guard.set_output_volume(volume).await {
                error!("Failed to set output volume: {}", e);
            }
        });
    };

    // Toggle audio enabled
    let toggle_audio_enabled = move |_| {
        let client = sip_client.clone();
        spawn(async move {
            let binding = client.peek();
            let client_guard = binding.read().await;
            
            let new_enabled = !(*audio_enabled.peek());
            match client_guard.set_audio_enabled(new_enabled).await {
                Ok(_) => {
                    info!("🎵 Audio enabled toggled: {}", new_enabled);
                    audio_enabled.set(new_enabled);
                }
                Err(e) => {
                    error!("Failed to toggle audio enabled: {}", e);
                }
            }
        });
    };

    // Get values for formatting
    let input_volume_percent = (*input_volume.read() * 100.0) as u32;
    let output_volume_percent = (*output_volume.read() * 100.0) as u32;
    let audio_active_value = *audio_active.read();
    let audio_enabled_value = *audio_enabled.read();
    let microphone_muted_value = *microphone_muted.read();
    let speaker_muted_value = *speaker_muted.read();
    let echo_cancellation_value = *echo_cancellation.read();
    let noise_suppression_value = *noise_suppression.read();
    let auto_gain_control_value = *auto_gain_control.read();
    
    rsx! {
        div {
            style: "
                background: white;
                border-radius: 12px;
                padding: 24px;
                box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
                border: 1px solid #E2E8F0;
                margin-bottom: 24px;
            ",
            
            div {
                style: "
                    margin-bottom: 20px;
                    padding-bottom: 16px;
                    border-bottom: 1px solid #F1F5F9;
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                ",
                
                h3 {
                    style: "
                        font-size: 1.125rem;
                        font-weight: 500;
                        color: #1E293B;
                        margin: 0;
                    ",
                    "Audio Controls"
                }
                
                div {
                    style: "
                        display: flex;
                        align-items: center;
                        gap: 8px;
                    ",
                    
                    div {
                        style: format!("
                            width: 12px;
                            height: 12px;
                            border-radius: 50%;
                            background: {};
                        ", if audio_active_value { "#10B981" } else { "#EF4444" }),
                    }
                    
                    span {
                        style: "
                            font-size: 0.875rem;
                            color: #64748B;
                            font-weight: 500;
                        ",
                        if audio_active_value { "Active" } else { "Inactive" }
                    }
                }
            }
            
            // Audio enable/disable
            div {
                style: "margin-bottom: 20px;",
                
                label {
                    style: "
                        display: flex;
                        align-items: center;
                        gap: 12px;
                        cursor: pointer;
                        font-size: 0.875rem;
                        color: #374151;
                        font-weight: 500;
                    ",
                    
                    input {
                        r#type: "checkbox",
                        checked: audio_enabled_value,
                        onchange: toggle_audio_enabled
                    }
                    
                    "Enable Audio"
                }
            }
            
            // Microphone controls
            div {
                style: "margin-bottom: 20px;",
                
                h4 {
                    style: "
                        font-size: 0.875rem;
                        font-weight: 500;
                        color: #374151;
                        margin: 0 0 12px 0;
                    ",
                    "🎤 Microphone"
                }
                
                div {
                    style: "
                        display: flex;
                        gap: 16px;
                        align-items: center;
                    ",
                    
                    button {
                        style: format!("
                            padding: 8px 16px;
                            background: {};
                            color: white;
                            border: none;
                            border-radius: 6px;
                            font-size: 0.875rem;
                            font-weight: 500;
                            cursor: pointer;
                            min-width: 80px;
                        ", if microphone_muted_value { "#EF4444" } else { "#10B981" }),
                        onclick: toggle_microphone_mute,
                        if microphone_muted_value { "🔇 Muted" } else { "🔊 Unmuted" }
                    }
                    
                    div {
                        style: "flex: 1;",
                        
                        label {
                            style: "
                                display: block;
                                font-size: 0.75rem;
                                color: #64748B;
                                margin-bottom: 4px;
                            ",
                            "Volume: {input_volume_percent}%"
                        }
                        
                        input {
                            r#type: "range",
                            min: "0",
                            max: "100",
                            value: "{input_volume_percent}",
                            onchange: set_input_volume,
                            style: "
                                width: 100%;
                                margin: 0;
                            "
                        }
                    }
                }
            }
            
            // Speaker controls
            div {
                style: "margin-bottom: 20px;",
                
                h4 {
                    style: "
                        font-size: 0.875rem;
                        font-weight: 500;
                        color: #374151;
                        margin: 0 0 12px 0;
                    ",
                    "🔊 Speaker"
                }
                
                div {
                    style: "
                        display: flex;
                        gap: 16px;
                        align-items: center;
                    ",
                    
                    button {
                        style: format!("
                            padding: 8px 16px;
                            background: {};
                            color: white;
                            border: none;
                            border-radius: 6px;
                            font-size: 0.875rem;
                            font-weight: 500;
                            cursor: pointer;
                            min-width: 80px;
                        ", if speaker_muted_value { "#EF4444" } else { "#10B981" }),
                        onclick: toggle_speaker_mute,
                        if speaker_muted_value { "🔇 Muted" } else { "🔊 Unmuted" }
                    }
                    
                    div {
                        style: "flex: 1;",
                        
                        label {
                            style: "
                                display: block;
                                font-size: 0.75rem;
                                color: #64748B;
                                margin-bottom: 4px;
                            ",
                            "Volume: {output_volume_percent}%"
                        }
                        
                        input {
                            r#type: "range",
                            min: "0",
                            max: "100",
                            value: "{output_volume_percent}",
                            onchange: set_output_volume,
                            style: "
                                width: 100%;
                                margin: 0;
                            "
                        }
                    }
                }
            }
            
            // Audio processing settings
            div {
                style: "
                    padding-top: 16px;
                    border-top: 1px solid #F1F5F9;
                ",
                
                h4 {
                    style: "
                        font-size: 0.875rem;
                        font-weight: 500;
                        color: #374151;
                        margin: 0 0 12px 0;
                    ",
                    "🎛️ Audio Processing"
                }
                
                div {
                    style: "
                        display: flex;
                        flex-direction: column;
                        gap: 8px;
                    ",
                    
                    label {
                        style: "
                            display: flex;
                            align-items: center;
                            gap: 8px;
                            cursor: pointer;
                            font-size: 0.75rem;
                            color: #64748B;
                        ",
                        
                        input {
                            r#type: "checkbox",
                            checked: echo_cancellation_value,
                            disabled: true // TODO: Implement toggle functionality
                        }
                        
                        "Echo Cancellation"
                    }
                    
                    label {
                        style: "
                            display: flex;
                            align-items: center;
                            gap: 8px;
                            cursor: pointer;
                            font-size: 0.75rem;
                            color: #64748B;
                        ",
                        
                        input {
                            r#type: "checkbox",
                            checked: noise_suppression_value,
                            disabled: true // TODO: Implement toggle functionality
                        }
                        
                        "Noise Suppression"
                    }
                    
                    label {
                        style: "
                            display: flex;
                            align-items: center;
                            gap: 8px;
                            cursor: pointer;
                            font-size: 0.75rem;
                            color: #64748B;
                        ",
                        
                        input {
                            r#type: "checkbox",
                            checked: auto_gain_control_value,
                            disabled: true // TODO: Implement toggle functionality
                        }
                        
                        "Automatic Gain Control"
                    }
                }
            }
            
            // Audio information
            if let Some(call_id) = call_id {
                div {
                    style: "
                        margin-top: 16px;
                        padding-top: 16px;
                        border-top: 1px solid #F1F5F9;
                    ",
                    
                    div {
                        style: "
                            font-size: 0.75rem;
                            color: #64748B;
                            display: flex;
                            justify-content: space-between;
                            margin-bottom: 4px;
                        ",
                        
                        span { "Call ID:" }
                        span { "{call_id}" }
                    }
                    
                    div {
                        style: "
                            font-size: 0.75rem;
                            color: #64748B;
                            display: flex;
                            justify-content: space-between;
                        ",
                        
                        span { "Audio Status:" }
                        span { 
                            style: format!("color: {}", if audio_active_value { "#10B981" } else { "#EF4444" }),
                            if audio_active_value { "Active" } else { "Inactive" }
                        }
                    }
                }
            }
        }
    }
} 