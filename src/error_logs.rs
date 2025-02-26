use leptos::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::app::invoke;

async fn get_error_logs() -> Option<Vec<String>> {
    serde_wasm_bindgen::from_value(invoke("command_error_logs", JsValue::null()).await).ok()
}

#[component]
pub fn ErrorLogs() -> impl IntoView {
    let (error_logs, set_error_logs) = signal(Vec::<String>::new());
    let (is_loading, set_is_loading) = signal(true);
    let (has_error, set_has_error) = signal(false);

    // Function to load logs
    let load_logs = move || {
        set_is_loading.set(true);
        set_has_error.set(false);

        spawn_local(async move {
            if let Some(logs) = get_error_logs().await {
                set_error_logs.set(logs);
                set_is_loading.set(false);
            } else {
                set_has_error.set(true);
                set_is_loading.set(false);
            }
        });
    };

    // Load logs when component mounts
    Effect::new(move |_| {
        load_logs();
    });
    view! {
        <div class="max-w-6xl mx-auto p-2 sm:p-4">
            <div class="bg-white rounded-lg shadow-lg p-4 sm:p-6 mb-8">
                <div class="flex justify-between items-center mb-6 border-b pb-4">
                    <h1 class="text-2xl font-bold text-gray-800">"Error Logs"</h1>
                    <button
                        on:click=move |_| load_logs()
                        class="bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded-md transition-colors duration-300 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-50"
                        disabled=is_loading
                    >
                        {move || if is_loading.get() { "Loading..." } else { "Refresh Logs" }}
                    </button>
                </div>

                <div>
                    {move || {
                        if is_loading.get() {
                            view! {
                                <div class="flex justify-center items-center h-64">
                                    <div class="animate-pulse text-gray-600">"Loading logs..."</div>
                                </div>
                            }
                                .into_any()
                        } else if has_error.get() {
                            view! {
                                <div class="bg-red-50 border-l-4 border-red-500 p-4 mb-4">
                                    <div class="flex">
                                        <div class="ml-3">
                                            <p class="text-red-700 font-medium">"Error loading logs"</p>
                                            <p class="text-red-600 mt-1">
                                                "There was a problem fetching the error logs. Please try again."
                                            </p>
                                        </div>
                                    </div>
                                </div>
                            }
                                .into_any()
                        } else if error_logs.get().is_empty() {
                            view! {
                                <div class="bg-gray-50 border border-gray-200 rounded-md p-6 text-center">
                                    <p class="text-gray-600">"No error logs found."</p>
                                </div>
                            }
                                .into_any()
                        } else {
                            view! {
                                <div>

                                    <textarea
                                        readonly
                                        class="border border-gray-300 rounded-md bg-gray-50 font-mono text-sm leading-relaxed text-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                                    >

                                        {error_logs.get().join("\n")}
                                    </textarea>
                                </div>
                            }
                                .into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
