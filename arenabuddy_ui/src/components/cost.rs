use arenabuddy_core::models::Cost;
use dioxus::prelude::*;

#[component]
pub fn ManaCost(cost: Cost) -> Element {
    rsx! {
        div { class: "flex items-center",
            for symbol in cost {
                img {
                    src: "/assets/mana/{symbol.svg_file()}",
                    alt: "{symbol}",
                    class: "w-4 h-4"
                }
            }
        }
    }
}
