use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::{
    debug_logs::DebugLogs, error_logs::ErrorLogs, match_details::MatchDetails, matches::Matches,
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    pub(crate) async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "opener"])]
    fn openUrl(url: &str) -> JsValue;
}

fn open_github() {
    openUrl("https://github.com/gazure/arenabuddy");
}

#[derive(Serialize, Deserialize)]
struct GreetArgs<'a> {
    name: &'a str,
}

#[component]
fn Home() -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow-md p-6">
            <h1 class="text-2xl font-bold mb-4 text-gray-800">"Home Page"</h1>
            <p class="text-gray-600">
                "Welcome to ArenaBuddy. Track and analyze your Arena matches."
            </p>
        </div>
    }
}

#[component]
fn Contact() -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow-md p-6">
            <h1 class="text-2xl font-bold mb-4 text-gray-800">"Contact"</h1>
            <a
                href="#"
                on:click=move |_| open_github()
                class="text-blue-600 hover:text-blue-800 transition-colors duration-200"
            >
                "Github Repo"
            </a>
        </div>
    }
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <nav class="bg-gray-800 p-4 shadow-md">
                <div class="container mx-auto">
                    <ul class="flex space-x-6 text-white">
                        <li>
                            <a
                                href="/"
                                class="hover:text-blue-400 transition-colors duration-200"
                                aria-current="page"
                            >
                                "Home"
                            </a>
                        </li>
                        <li>
                            <a
                                href="/matches"
                                class="hover:text-blue-400 transition-colors duration-200"
                            >
                                "Matches"
                            </a>
                        </li>
                        <li>
                            <a
                                href="/errors"
                                class="hover:text-blue-400 transition-colors duration-200"
                            >
                                "Error Logs"
                            </a>
                        </li>
                        <li>
                            <a
                                href="/debug"
                                class="hover:text-blue-400 transition-colors duration-200"
                            >
                                "Debug Logs"
                            </a>
                        </li>
                        <li>
                            <a
                                href="/contact"
                                class="hover:text-blue-400 transition-colors duration-200"
                            >
                                "Contact"
                            </a>
                        </li>
                    </ul>
                </div>
            </nav>
            <main class="container mx-auto p-4">
                <Routes fallback=|| {
                    view! {
                        <div class="text-center mt-8">
                            <h1 class="text-2xl font-bold text-red-600">"Page Not Found"</h1>
                            <p class="mt-2 text-gray-600">
                                "The page you're looking for doesn't exist."
                            </p>
                        </div>
                    }
                }>
                    <Route path=path!("/") view=Home />
                    <Route path=path!("/matches") view=Matches />
                    <Route path=path!("/errors") view=ErrorLogs />
                    <Route path=path!("/contact") view=Contact />
                    <Route path=path!("/match/:id") view=MatchDetails />
                    <Route path=path!("/debug") view=DebugLogs />
                </Routes>
            </main>
        </Router>
    }
}
