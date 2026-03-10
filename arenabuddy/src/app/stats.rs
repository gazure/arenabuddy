use arenabuddy_core::display::stats::{MatchStats, TimeWindow};
use dioxus::prelude::*;

use crate::backend::Service;

fn format_rate(rate: Option<f64>) -> String {
    rate.map_or("N/A".to_string(), |r| format!("{r:.1}%"))
}

#[component]
fn StatCard(title: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "bg-gray-800 rounded-lg border border-gray-700 p-6",
            h2 { class: "text-lg font-semibold text-gray-300 mb-4", "{title}" }
            {children}
        }
    }
}

#[component]
fn RecordLine(label: &'static str, wins: i64, losses: i64, rate: Option<f64>) -> Element {
    rsx! {
        div { class: "flex justify-between items-center py-2 border-b border-gray-700 last:border-0",
            span { class: "text-gray-400", "{label}" }
            div { class: "flex items-center space-x-3",
                span { class: "text-amber-400 font-medium", "{wins}W" }
                span { class: "text-gray-600", "-" }
                span { class: "text-red-400 font-medium", "{losses}L" }
                span { class: "text-gray-500 text-sm ml-2", "({format_rate(rate)})" }
            }
        }
    }
}

#[component]
fn StatsDisplay(stats: MatchStats) -> Element {
    rsx! {
        div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
            StatCard { title: "Match Record",
                RecordLine {
                    label: "Overall",
                    wins: stats.match_wins,
                    losses: stats.match_losses,
                    rate: stats.match_win_rate(),
                }
                div { class: "pt-2 text-sm text-gray-500",
                    "{stats.total_matches} matches played"
                }
            }

            StatCard { title: "Game Record",
                RecordLine {
                    label: "Overall",
                    wins: stats.game_wins,
                    losses: stats.game_losses,
                    rate: stats.game_win_rate(),
                }
                div { class: "pt-2 text-sm text-gray-500",
                    "{stats.total_games} games played"
                }
            }

            StatCard { title: "Play / Draw Game Win Rate",
                RecordLine {
                    label: "On the Play",
                    wins: stats.play_wins,
                    losses: stats.play_losses,
                    rate: stats.play_win_rate(),
                }
                RecordLine {
                    label: "On the Draw",
                    wins: stats.draw_wins,
                    losses: stats.draw_losses,
                    rate: stats.draw_win_rate(),
                }
            }

            StatCard { title: "Mulligans",
                if stats.mulligan_stats.is_empty() {
                    p { class: "text-gray-500 text-sm", "No mulligan data available" }
                } else {
                    for bucket in stats.mulligan_stats.iter() {
                        RecordLine {
                            label: match bucket.cards_kept {
                                7 => "Kept 7",
                                6 => "Kept 6",
                                5 => "Kept 5",
                                4 => "Kept 4",
                                _ => "Other",
                            },
                            wins: bucket.wins,
                            losses: bucket.losses,
                            rate: bucket.win_rate(),
                        }
                    }
                }
            }

            if !stats.opponents.is_empty() {
                StatCard { title: "Top Opponents",
                    for opp in stats.opponents.iter() {
                        div { class: "flex justify-between items-center py-2 border-b border-gray-700 last:border-0",
                            span { class: "text-gray-400 truncate mr-4", "{opp.name}" }
                            div { class: "flex items-center space-x-3 flex-shrink-0",
                                span { class: "text-amber-400 font-medium", "{opp.wins}W" }
                                span { class: "text-gray-600", "-" }
                                span { class: "text-red-400 font-medium", "{opp.losses}L" }
                                span { class: "text-gray-500 text-sm ml-2",
                                    "({opp.matches} matches)"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub(crate) fn Stats() -> Element {
    let service = use_context::<Service>();
    let mut time_window = use_signal(TimeWindow::default);

    let mut stats_resource = use_resource(move || {
        let service = service.clone();
        let tw = time_window();
        async move { service.get_stats(tw).await }
    });

    let refresh = move |_| {
        stats_resource.restart();
    };

    let resource_value = stats_resource.value();
    let data = resource_value.read();

    rsx! {
        div { class: "container mx-auto px-4 py-8 max-w-5xl",
            div { class: "flex justify-between items-center mb-6",
                h1 { class: "text-2xl font-bold text-gray-100", "Match Statistics" }
                div { class: "flex items-center space-x-3",
                    select {
                        class: "bg-gray-700 text-gray-200 border border-gray-600 rounded py-2 px-3 text-sm focus:outline-none focus:border-amber-500",
                        onchange: move |evt| {
                            let idx: usize = evt.value().parse().unwrap_or(3);
                            time_window.set(TimeWindow::ALL[idx]);
                        },
                        for (i, tw) in TimeWindow::ALL.iter().enumerate() {
                            option {
                                value: "{i}",
                                selected: *tw == time_window(),
                                "{tw.label()}"
                            }
                        }
                    }
                    button {
                        onclick: refresh,
                        class: "bg-amber-600 hover:bg-amber-700 text-white py-2 px-4 rounded transition-colors duration-150",
                        disabled: data.is_none(),
                        if data.is_none() {
                            "Loading..."
                        } else {
                            "Refresh"
                        }
                    }
                }
            }

            match &*data {
                None => rsx! {
                    div { class: "bg-gray-800 rounded-lg border border-gray-700 p-12 text-center text-gray-500",
                        div { class: "animate-pulse", "Loading statistics..." }
                    }
                },

                Some(Err(err)) => rsx! {
                    div { class: "bg-red-900/30 border border-red-700 text-red-300 px-4 py-3 rounded",
                        p { "Failed to load statistics: {err}" }
                    }
                },

                Some(Ok(stats)) => {
                    if stats.total_matches == 0 {
                        rsx! {
                            div { class: "bg-gray-800 rounded-lg border border-gray-700 p-12 text-center text-gray-500",
                                "No match data available. Play some games in MTG Arena!"
                            }
                        }
                    } else {
                        rsx! { StatsDisplay { stats: stats.clone() } }
                    }
                }
            }
        }
    }
}
