use dioxus::prelude::*;

#[component]
pub fn IncomingCallScreen(
    caller_id: String,
    on_answer: EventHandler<()>,
    on_ignore: EventHandler<()>
) -> Element {
    rsx! {
        div {
            style: "
                background: white;
                border-radius: 12px;
                padding: 48px 32px;
                box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
                border: 1px solid #E2E8F0;
                text-align: center;
                animation: pulse 2s infinite;
            ",
            
            div {
                style: "margin-bottom: 32px;",
                
                h2 {
                    style: "
                        font-size: 1.75rem;
                        font-weight: 500;
                        color: #1E293B;
                        margin: 0 0 16px 0;
                    ",
                    "Incoming Call"
                }
                
                p {
                    style: "
                        font-size: 1.125rem;
                        color: #059669;
                        margin: 0;
                        font-weight: 500;
                    ",
                    "{caller_id}"
                }
            }
            
            div {
                style: "display: flex; gap: 16px; justify-content: center;",
                
                button {
                    style: "
                        padding: 16px 24px;
                        background: #059669;
                        color: white;
                        border: none;
                        border-radius: 8px;
                        font-size: 1rem;
                        font-weight: 500;
                        cursor: pointer;
                        min-width: 120px;
                    ",
                    onclick: move |_| on_answer.call(()),
                    "Answer"
                }
                
                button {
                    style: "
                        padding: 16px 24px;
                        background: #DC2626;
                        color: white;
                        border: none;
                        border-radius: 8px;
                        font-size: 1rem;
                        font-weight: 500;
                        cursor: pointer;
                        min-width: 120px;
                    ",
                    onclick: move |_| on_ignore.call(()),
                    "Ignore"
                }
            }
        }
    }
} 