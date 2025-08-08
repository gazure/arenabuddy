use dioxus::prelude::*;

use crate::service::command_error_logs;

async fn get_error_logs() -> Option<Vec<String>> {
    command_error_logs().await.ok()
}

#[component]
pub fn ErrorLogs() -> Element {
    let mut error_logs = use_signal(Vec::<String>::new);
    let mut is_loading = use_signal(|| true);
    let mut has_error = use_signal(|| false);

    // Function to load logs
    let mut load_logs = move || {
        is_loading.set(true);
        has_error.set(false);

        spawn(async move {
            if let Some(logs) = get_error_logs().await {
                error_logs.set(logs);
                is_loading.set(false);
            } else {
                has_error.set(true);
                is_loading.set(false);
            }
        });
    };

    // Load logs when component mounts
    use_effect(move || {
        load_logs();
    });

    rsx! {
        div { class: "max-w-6xl mx-auto p-2 sm:p-4",
            div { class: "bg-white rounded-lg shadow-lg p-4 sm:p-6 mb-8",
                div { class: "flex justify-between items-center mb-6 border-b pb-4",
                    h1 { class: "text-2xl font-bold text-gray-800", "Error Logs" }
                    button {
                        onclick: move |_| load_logs(),
                        class: "bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded-md transition-colors duration-300 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-50",
                        disabled: is_loading(),
                        if is_loading() { "Loading..." } else { "Refresh Logs" }
                    }
                }

                div {
                    if is_loading() {
                        div { class: "flex justify-center items-center h-64",
                            div { class: "animate-pulse text-gray-600", "Loading logs..." }
                        }
                    } else if has_error() {
                        div { class: "bg-red-50 border-l-4 border-red-500 p-4 mb-4",
                            div { class: "flex",
                                div { class: "ml-3",
                                    p { class: "text-red-700 font-medium", "Error loading logs" }
                                    p { class: "text-red-600 mt-1",
                                        "There was a problem fetching the error logs. Please try again."
                                    }
                                }
                            }
                        }
                    } else if error_logs().is_empty() {
                        div { class: "bg-gray-50 border border-gray-200 rounded-md p-6 text-center",
                            p { class: "text-gray-600", "No error logs found." }
                        }
                    } else {
                        div {
                            textarea {
                                readonly: true,
                                class: "border border-gray-300 rounded-md bg-gray-50 font-mono text-sm leading-relaxed text-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 w-full",
                                value: "{error_logs().join(\"\\n\")}"
                            }
                        }
                    }
                }
            }
        }
    }
}
