use dioxus::prelude::*;

#[component]
pub fn UserInfoBar(
    username: String,
    status_text: String,
    on_logout: EventHandler<()>
) -> Element {
    rsx! {
        div {
            class: "bg-white rounded-xl px-6 py-4 shadow-sm border border-gray-200 flex justify-between items-center",
            
            div {
                div {
                    class: "font-medium text-gray-800 text-sm",
                    "Connected as: {username}"
                }
                div {
                    class: "text-gray-500 text-xs mt-0.5",
                    "{status_text}"
                }
            }
            
            button {
                class: "px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white rounded-md text-xs font-medium transition-colors",
                onclick: move |_| on_logout.call(()),
                "Logout"
            }
        }
    }
}