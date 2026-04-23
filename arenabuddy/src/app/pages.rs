use dioxus::prelude::*;
use dioxus_router::{Link, Outlet, Routable};

use crate::{
    app::{
        debug_logs::DebugLogs, draft_details::DraftDetails, drafts::Drafts, error_logs::ErrorLogs,
        match_details::MatchDetails, matches::Matches, stats::Stats,
    },
    backend::{BackgroundRuntime, Service, SharedAuthState, auth_controller},
};

fn open_github() {
    if let Err(e) = open::that("https://github.com/gazure/arenabuddy") {
        tracing::error!("Failed to open URL: {}", e);
    }
}

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
        #[route("/drafts")]
        Drafts {},
        #[route("/drafts/:id")]
        DraftDetails { id: String },
        #[route("/debug")]
        DebugLogs {},
        #[route("/stats")]
        Stats {},
    #[end_layout]
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}

#[component]
fn Home() -> Element {
    rsx! {
        div { class: "bg-gray-800 rounded-lg border border-gray-700 p-6",
            h1 { class: "text-2xl font-bold mb-4 text-gray-100", "Home Page" }
            p { class: "text-gray-400",
                "Welcome to ArenaBuddy. Track and analyze your Arena matches."
            }
        }
    }
}

#[component]
fn Contact() -> Element {
    rsx! {
        div { class: "bg-gray-800 rounded-lg border border-gray-700 p-6",
            h1 { class: "text-2xl font-bold mb-4 text-gray-100", "Contact" }
            a {
                href: "#",
                onclick: move |_| open_github(),
                class: "text-amber-400 hover:text-amber-300 transition-colors duration-200",
                "Github Repo"
            }
        }
    }
}

#[component]
fn PageNotFound(route: Vec<String>) -> Element {
    rsx! {
        div { class: "text-center mt-8",
            h1 { class: "text-2xl font-bold text-red-400", "Page Not Found" }
            p { class: "mt-2 text-gray-400",
                "The page you're looking for doesn't exist."
            }
        }
    }
}

#[component]
fn Layout() -> Element {
    let auth_state = use_context::<SharedAuthState>();
    let mut login_status = use_signal(|| None::<String>);
    let mut login_loading = use_signal(|| false);

    // Check current auth state on render
    let auth_state_effect = auth_state.clone();
    use_effect(move || {
        let auth_state = auth_state_effect.clone();
        spawn(async move {
            let state = auth_state.lock().await;
            login_status.set(state.as_ref().map(|s| s.user.username.clone()));
        });
    });

    let bg_runtime = use_context::<BackgroundRuntime>();
    let service = use_context::<Service>();
    let on_login = {
        let auth_state = auth_state.clone();
        let bg_runtime = bg_runtime.clone();
        let service = service.clone();
        move |_| {
            let auth_state = auth_state.clone();
            let bg = bg_runtime.clone();
            let service = service.clone();
            spawn(async move {
                login_loading.set(true);
                match auth_controller::login(auth_state, service, bg).await {
                    Ok(outcome) => login_status.set(Some(outcome.username)),
                    Err(e) => tracing::error!("Login failed: {e}"),
                }
                login_loading.set(false);
            });
        }
    };

    let on_logout = {
        let auth_state = auth_state.clone();
        let bg_runtime = bg_runtime.clone();
        move |_| {
            let auth_state = auth_state.clone();
            let bg = bg_runtime.clone();
            spawn(async move {
                match auth_controller::logout(auth_state, bg).await {
                    Ok(()) => login_status.set(None),
                    Err(e) => tracing::error!("Logout failed: {e}"),
                }
            });
        }
    };

    rsx! {
        nav { class: "bg-gray-950 p-4 border-b border-gray-800",
            div { class: "container mx-auto flex justify-between items-center",
                ul { class: "flex space-x-6 text-white",
                    li {
                        Link {
                            to: Route::Home {},
                            class: "hover:text-amber-400 transition-colors duration-200",
                            "Home"
                        }
                    }
                    li {
                        Link {
                            to: Route::Matches {},
                            class: "hover:text-amber-400 transition-colors duration-200",
                            "Matches"
                        }
                    }
                    li {
                        Link {
                            to: Route::Drafts { },
                            class: "hover:text-amber-400 transition-colors duration-200",
                            "Drafts"
                        }
                    }
                    li {
                        Link {
                            to: Route::Stats {},
                            class: "hover:text-amber-400 transition-colors duration-200",
                            "Stats"
                        }
                    }
                    li {
                        Link {
                            to: Route::ErrorLogs {},
                            class: "hover:text-amber-400 transition-colors duration-200",
                            "Error Logs"
                        }
                    }
                    li {
                        Link {
                            to: Route::DebugLogs {},
                            class: "hover:text-amber-400 transition-colors duration-200",
                            "Debug Logs"
                        }
                    }
                    li {
                        Link {
                            to: Route::Contact {},
                            class: "hover:text-amber-400 transition-colors duration-200",
                            "Contact"
                        }
                    }
                }
                div { class: "text-white flex items-center space-x-3",
                    if let Some(username) = login_status() {
                        span { class: "text-emerald-400 text-sm", "Logged in as {username}" }
                        button {
                            class: "bg-red-600 hover:bg-red-700 text-white text-sm px-3 py-1 rounded transition-colors duration-200",
                            onclick: on_logout,
                            "Logout"
                        }
                    } else if login_loading() {
                        span { class: "text-yellow-400 text-sm", "Logging in..." }
                    } else {
                        button {
                            class: "bg-violet-600 hover:bg-violet-700 text-white text-sm px-3 py-1 rounded transition-colors duration-200",
                            onclick: on_login,
                            "Login with Discord"
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
