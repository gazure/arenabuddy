use crate::matches::Matches;
use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

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

// Component for Home page
#[component]
fn Home() -> impl IntoView {
    view! {
        <div>
            <h1>"Home Page"</h1>
        // Add your home page content here
        </div>
    }
}

// Component for Matches page
#[component]
fn Contact() -> impl IntoView {
    view! {
        <div>
            <h1>"Contact"</h1>
            <a href="#" on:click=move |_| open_github()>
                Github Repo
            </a>
        </div>
    }
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <nav class="bg-white border-b border-gray-200 dark:bg-gray-900 dark:border-gray-700">
                <div class="hidden w-full md:block md:w-auto" id="navbar-default">
                    <ul class="font-medium flex flex-col p-4 md:p-0 mt-4 border border-gray-100 rounded-lg bg-gray-50 md:flex-row md:space-x-8 rtl:space-x-reverse md:mt-0 md:border-0 md:bg-white dark:bg-gray-800 md:dark:bg-gray-900 dark:border-gray-700">
                        <li>
                            <a
                                href="/"
                                class="block py-2 px-3 text-white bg-blue-700 rounded md:bg-transparent md:text-blue-700 md:p-0 dark:text-white md:dark:text-blue-500"
                                aria-current="page"
                            >
                                "Home"
                            </a>
                        </li>
                        <li>
                            <a
                                href="/matches"
                                class="block py-2 px-3 text-gray-900 rounded hover:bg-gray-100 md:hover:bg-transparent md:border-0 md:hover:text-blue-700 md:p-0 dark:text-white md:dark:hover:text-blue-500 dark:hover:bg-gray-700 dark:hover:text-white md:dark:hover:bg-transparent"
                            >
                                "Matches"
                            </a>
                        </li>
                        <li>
                            <a
                                href="/contact"
                                class="block py-2 px-3 text-gray-900 rounded hover:bg-gray-100 md:hover:bg-transparent md:border-0 md:hover:text-blue-700 md:p-0 dark:text-white md:dark:hover:text-blue-500 dark:hover:bg-gray-700 dark:hover:text-white md:dark:hover:bg-transparent"
                            >
                                "Contact"
                            </a>
                        </li>
                    </ul>
                </div>
            </nav>
            <main>
                <Routes fallback=|| "Not found.">
                    <Route path=path!("/") view=Home />
                    <Route path=path!("/matches") view=Matches />
                    <Route path=path!("/contact") view=Contact />
                </Routes>
            </main>
        </Router>
    }
}
