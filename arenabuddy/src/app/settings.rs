use dioxus::prelude::*;
use zeroize::Zeroizing;

use crate::backend::{BackgroundRuntime, credentials};

/// Renders the desktop credential settings without revealing saved keys.
#[component]
pub fn Settings() -> Element {
    let runtime = use_context::<BackgroundRuntime>();
    let mut input = use_signal(|| Zeroizing::new(String::new()));
    let mut busy = use_signal(|| false);
    let mut message = use_signal(|| None::<String>);
    let status_runtime = runtime.clone();
    let mut status = use_resource(move || {
        let runtime = status_runtime.clone();
        async move {
            runtime
                .spawn_blocking(|| credentials::load_typesafe_key().map(|key| key.is_some()))
                .await
                .unwrap_or(Err(credentials::CredentialError::Unavailable))
        }
    });
    let save_runtime = runtime.clone();
    let save = move |_| {
        if busy() {
            return;
        }
        busy.set(true);
        message.set(None);
        let key = std::mem::replace(&mut *input.write(), Zeroizing::new(String::new()));
        let runtime = save_runtime.clone();
        spawn(async move {
            let result = runtime
                .spawn_blocking(move || credentials::save_typesafe_key(&key))
                .await
                .unwrap_or(Err(credentials::CredentialError::Unavailable));
            message.set(Some(match result {
                Ok(()) => "API key saved securely. API access has not been checked.".to_string(),
                Err(error) => error.to_string(),
            }));
            status.restart();
            busy.set(false);
        });
    };
    let remove = move |_| {
        if busy() {
            return;
        }
        busy.set(true);
        input.set(Zeroizing::new(String::new()));
        message.set(None);
        let runtime = runtime.clone();
        spawn(async move {
            let result = runtime
                .spawn_blocking(credentials::remove_typesafe_key)
                .await
                .unwrap_or(Err(credentials::CredentialError::Unavailable));
            message.set(Some(match result {
                Ok(()) => "Saved API key removed.".to_string(),
                Err(error) => error.to_string(),
            }));
            status.restart();
            busy.set(false);
        });
    };
    let saved = status.read().as_ref().copied();

    rsx! {
        div { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6",
            h1 { class: "text-2xl font-bold mb-4 text-gray-900 dark:text-gray-100", "Settings" }
            h2 { class: "text-lg font-semibold mb-4 text-gray-900 dark:text-gray-100", "TypeSafe API key" }
            p { class: "mb-4 text-gray-600 dark:text-gray-400",
                "Save your key in your operating system’s credential store for Jev features. Saving a new key replaces the existing key."
            }
            p { role: "status", class: "mb-4 text-gray-600 dark:text-gray-400",
                {match saved {
                    None => "Checking secure storage…".to_string(),
                    Some(Ok(true)) => "An API key is saved.".to_string(),
                    Some(Ok(false)) => "No API key is saved.".to_string(),
                    Some(Err(error)) => error.to_string(),
                }}
            }
            label { r#for: "typesafe-key", class: "block mb-2 text-gray-900 dark:text-gray-100", "New API key" }
            input {
                id: "typesafe-key",
                r#type: "password",
                autocomplete: "off",
                spellcheck: "false",
                class: "w-full p-2 mb-4 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100",
                value: "{input.read().as_str()}",
                disabled: busy(),
                oninput: move |event| input.set(Zeroizing::new(event.value())),
            }
            div { class: "flex space-x-3",
                button {
                    class: "bg-violet-600 hover:bg-violet-700 text-white px-3 py-1 rounded",
                    disabled: busy() || input.read().trim().is_empty(),
                    onclick: save,
                    "Save key"
                }
                button {
                    class: "bg-red-600 hover:bg-red-700 text-white px-3 py-1 rounded",
                    disabled: busy() || !matches!(saved, Some(Ok(true))),
                    onclick: remove,
                    "Remove key"
                }
                button {
                    class: "text-gray-900 dark:text-gray-100 px-3 py-1 rounded",
                    disabled: busy(),
                    onclick: move |_| status.restart(),
                    "Refresh status"
                }
            }
            if busy() { p { role: "status", "Waiting for secure storage…" } }
            if let Some(text) = message() {
                p { role: "status", class: "mt-4 text-gray-900 dark:text-gray-100", "{text}" }
            }
        }
    }
}
