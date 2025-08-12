use dioxus::prelude::*;
use dioxus_router::{Link, Outlet, Routable};

use crate::{debug_logs::DebugLogs, error_logs::ErrorLogs, match_details::MatchDetails, matches::Matches};

fn open_github() {
    if let Err(e) = open::that("https://github.com/gazure/monorepo") {
        tracing::error!("Failed to open URL: {}", e);
    }
}

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Layout)]
        #[route("/")]
        Home {},
        #[route("/matches")]
        Matches {},
        #[route("/errors")]
        ErrorLogs {},
        #[route("/contact")]
        Contact {},
        #[route("/match/:id")]
        MatchDetails{ id: String },
        #[route("/debug")]
        DebugLogs {},
    #[end_layout]
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}

#[component]
fn Home() -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            h1 { class: "text-2xl font-bold mb-4 text-gray-800", "Home Page" }
            p { class: "text-gray-600",
                "Welcome to ArenaBuddy. Track and analyze your Arena matches."
            }
        }
    }
}

#[component]
fn Contact() -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            h1 { class: "text-2xl font-bold mb-4 text-gray-800", "Contact" }
            a {
                href: "#",
                onclick: move |_| open_github(),
                class: "text-blue-600 hover:text-blue-800 transition-colors duration-200",
                "Github Repo"
            }
        }
    }
}

#[component]
fn PageNotFound(route: Vec<String>) -> Element {
    rsx! {
        div { class: "text-center mt-8",
            h1 { class: "text-2xl font-bold text-red-600", "Page Not Found" }
            p { class: "mt-2 text-gray-600",
                "The page you're looking for doesn't exist."
            }
        }
    }
}

#[component]
pub fn App() -> Element {
    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        Router::<Route> {}
    }
}

#[component]
fn Layout() -> Element {
    rsx! {
        nav { class: "bg-gray-800 p-4 shadow-md",
            div { class: "container mx-auto",
                ul { class: "flex space-x-6 text-white",
                    li {
                        Link {
                            to: Route::Home {},
                            class: "hover:text-blue-400 transition-colors duration-200",
                            "Home"
                        }
                    }
                    li {
                        Link {
                            to: Route::Matches {},
                            class: "hover:text-blue-400 transition-colors duration-200",
                            "Matches"
                        }
                    }
                    li {
                        Link {
                            to: Route::ErrorLogs {},
                            class: "hover:text-blue-400 transition-colors duration-200",
                            "Error Logs"
                        }
                    }
                    li {
                        Link {
                            to: Route::DebugLogs {},
                            class: "hover:text-blue-400 transition-colors duration-200",
                            "Debug Logs"
                        }
                    }
                    li {
                        Link {
                            to: Route::Contact {},
                            class: "hover:text-blue-400 transition-colors duration-200",
                            "Contact"
                        }
                    }
                }
            }
        }
        main { class: "container mx-auto p-4",
            Outlet::<Route> {}
        }
    }
}
