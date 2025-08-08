use dioxus::prelude::*;

use crate::service::{command_get_debug_logs, command_set_debug_logs};

async fn set_debug_logs_dir(directory: &str) -> Result<(), String> {
    command_set_debug_logs(directory.to_string())
        .await
        .map_err(|e| e.to_string())
}

async fn get_debug_logs_dir() -> Result<Option<Vec<String>>, String> {
    command_get_debug_logs().await.map_err(|e| e.to_string())
}

async fn select_directory() -> Result<String, String> {
    use rfd::AsyncFileDialog;

    let folder = AsyncFileDialog::new()
        .set_title("Select Debug Logs Directory")
        .pick_folder()
        .await;

    match folder {
        Some(path) => Ok(path.path().to_string_lossy().to_string()),
        None => Err("No directory selected".to_string()),
    }
}

#[component]
pub fn DebugLogs() -> Element {
    let mut selected_dir = use_signal(|| Option::<Vec<String>>::None);
    let mut status_message = use_signal(|| Option::<String>::None);
    let mut is_loading = use_signal(|| false);
    let mut is_initial_load = use_signal(|| true);

    // Load current debug logs directory on startup
    use_effect(move || {
        spawn(async move {
            match get_debug_logs_dir().await {
                Ok(Some(logs)) => {
                    selected_dir.set(Some(logs));
                    status_message.set(Some("Loaded current debug logs".to_string()));
                }
                Ok(None) => {
                    status_message.set(Some("No debug logs directory configured yet".to_string()));
                }
                Err(err) => {
                    status_message.set(Some(format!("Error loading debug logs: {err}")));
                }
            }
            is_initial_load.set(false);
        });
    });

    let on_select_directory = move |_| {
        is_loading.set(true);
        status_message.set(None);

        spawn(async move {
            match select_directory().await {
                Ok(dir) => {
                    match set_debug_logs_dir(&dir).await {
                        Ok(()) => {
                            // Reload the logs after setting directory
                            match get_debug_logs_dir().await {
                                Ok(Some(logs)) => {
                                    selected_dir.set(Some(logs));
                                    status_message.set(Some(
                                        "Debug logs directory updated successfully!".to_string(),
                                    ));
                                }
                                Ok(None) => {
                                    status_message
                                        .set(Some("Directory set but no logs found".to_string()));
                                }
                                Err(err) => {
                                    status_message.set(Some(format!(
                                        "Directory set but error loading logs: {err}"
                                    )));
                                }
                            }
                        }
                        Err(err) => {
                            status_message.set(Some(format!("Error setting directory: {err}")));
                        }
                    }
                }
                Err(err) => {
                    status_message.set(Some(format!("Error selecting directory: {err}")));
                }
            }
            is_loading.set(false);
        });
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            h1 { class: "text-2xl font-bold mb-4 text-gray-800", "Debug Logs Configuration" }

            div { class: "mb-6",
                p { class: "text-gray-600 mb-4",
                    "Select a directory where debug logs will be saved. This helps with troubleshooting and debugging Arena Buddy."
                }

                button {
                    onclick: on_select_directory,
                    disabled: is_loading() || is_initial_load(),
                    class: "bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white font-medium py-2 px-4 rounded-lg transition-colors duration-200",
                    if is_initial_load() {
                        "Loading..."
                    } else if is_loading() {
                        "Selecting..."
                    } else if selected_dir().is_some() {
                        "Change Directory"
                    } else {
                        "Select Directory"
                    }
                }
            }

            if let Some(logs) = selected_dir() {
                div { class: "mb-4 p-3 bg-gray-100 rounded-lg",
                    p { class: "text-sm font-medium text-gray-700",
                        "Debug Logs ({logs.len()} entries):"
                    }
                    div { class: "max-h-48 overflow-y-auto",
                        for log in logs {
                            p { class: "text-sm text-gray-600 break-all font-mono", "{log}" }
                        }
                    }
                }
            }

            if let Some(msg) = status_message() {
                {
                    let is_error = msg.contains("Error");
                    let is_info = msg.contains("Loaded current")
                        || msg.contains("No debug logs directory configured");
                    let class = if is_error {
                        "p-3 bg-red-100 border border-red-400 text-red-700 rounded-lg"
                    } else if is_info {
                        "p-3 bg-blue-100 border border-blue-400 text-blue-700 rounded-lg"
                    } else {
                        "p-3 bg-green-100 border border-green-400 text-green-700 rounded-lg"
                    };

                    rsx! {
                        div { class: "{class}",
                            p { class: "text-sm", "{msg}" }
                        }
                    }
                }
            }
        }
    }
}
