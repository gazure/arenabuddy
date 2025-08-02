use dioxus::prelude::*;
use wasm_bindgen::{JsValue, prelude::*};

use crate::app::invoke;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"])]
    async fn open(options: JsValue) -> JsValue;
}

async fn set_debug_logs_dir(directory: &str) -> Result<(), String> {
    let result = invoke(
        "command_set_debug_logs",
        serde_wasm_bindgen::to_value(&serde_json::json!({"dir": directory})).unwrap(),
    )
    .await;

    // Check if the result indicates an error
    if result.is_null() || result.is_undefined() {
        Ok(())
    } else {
        // Try to parse error from result
        match serde_wasm_bindgen::from_value::<String>(result) {
            Ok(error_msg) => Err(error_msg),
            Err(_) => Ok(()), // If we can't parse it as an error, assume success
        }
    }
}

async fn get_debug_logs_dir() -> Result<Option<String>, String> {
    let result = invoke(
        "command_get_debug_logs",
        serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap(),
    )
    .await;

    // Check if the result indicates an error
    if result.is_null() || result.is_undefined() {
        Ok(None)
    } else {
        // Try to parse the result
        match serde_wasm_bindgen::from_value::<Option<String>>(result) {
            Ok(dir) => Ok(dir),
            Err(_) => Err("Failed to parse debug logs directory".to_string()),
        }
    }
}

async fn select_directory() -> Result<String, String> {
    let options = serde_json::json!({
        "directory": true,
        "multiple": false,
        "title": "Select Debug Logs Directory"
    });

    let result = open(serde_wasm_bindgen::to_value(&options).unwrap()).await;

    if result.is_null() {
        return Err("No directory selected".to_string());
    }

    serde_wasm_bindgen::from_value::<String>(result)
        .map_err(|_| "Failed to parse selected directory".to_string())
}

#[component]
pub fn DebugLogs() -> Element {
    let mut selected_dir = use_signal(|| Option::<String>::None);
    let mut status_message = use_signal(|| Option::<String>::None);
    let mut is_loading = use_signal(|| false);
    let mut is_initial_load = use_signal(|| true);

    // Load current debug logs directory on startup
    use_effect(move || {
        spawn(async move {
            match get_debug_logs_dir().await {
                Ok(Some(dir)) => {
                    selected_dir.set(Some(dir));
                    status_message.set(Some("Loaded current debug logs directory".to_string()));
                }
                Ok(None) => {
                    status_message
                        .set(Some("No debug logs directory configured yet".to_string()));
                }
                Err(err) => {
                    status_message.set(Some(format!("Error loading current directory: {err}")));
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
                    selected_dir.set(Some(dir.clone()));
                    match set_debug_logs_dir(&dir).await {
                        Ok(()) => {
                            status_message.set(Some(
                                "Debug logs directory updated successfully!".to_string(),
                            ));
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

            if let Some(dir) = selected_dir() {
                div { class: "mb-4 p-3 bg-gray-100 rounded-lg",
                    p { class: "text-sm font-medium text-gray-700",
                        "Debug Logs Directory:"
                    }
                    p { class: "text-sm text-gray-600 break-all", "{dir}" }
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