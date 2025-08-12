use arenabuddy_core::models::{Color, Cost, CostSymbol};
use dioxus::prelude::*;

// Asset constants for all mana symbols
const MANA_0: Asset = asset!("/assets/mana/0.svg");
const MANA_1: Asset = asset!("/assets/mana/1.svg");
const MANA_2: Asset = asset!("/assets/mana/2.svg");
const MANA_3: Asset = asset!("/assets/mana/3.svg");
const MANA_4: Asset = asset!("/assets/mana/4.svg");
const MANA_5: Asset = asset!("/assets/mana/5.svg");
const MANA_6: Asset = asset!("/assets/mana/6.svg");
const MANA_7: Asset = asset!("/assets/mana/7.svg");
const MANA_8: Asset = asset!("/assets/mana/8.svg");
const MANA_9: Asset = asset!("/assets/mana/9.svg");
const MANA_10: Asset = asset!("/assets/mana/10.svg");
const MANA_11: Asset = asset!("/assets/mana/11.svg");
const MANA_12: Asset = asset!("/assets/mana/12.svg");
const MANA_13: Asset = asset!("/assets/mana/13.svg");
const MANA_14: Asset = asset!("/assets/mana/14.svg");
const MANA_15: Asset = asset!("/assets/mana/15.svg");
const MANA_16: Asset = asset!("/assets/mana/16.svg");
const MANA_17: Asset = asset!("/assets/mana/17.svg");
const MANA_18: Asset = asset!("/assets/mana/18.svg");
const MANA_19: Asset = asset!("/assets/mana/19.svg");
const MANA_20: Asset = asset!("/assets/mana/20.svg");

// Basic mana colors
const MANA_W: Asset = asset!("/assets/mana/W.svg");
const MANA_U: Asset = asset!("/assets/mana/U.svg");
const MANA_B: Asset = asset!("/assets/mana/B.svg");
const MANA_R: Asset = asset!("/assets/mana/R.svg");
const MANA_G: Asset = asset!("/assets/mana/G.svg");

// Special mana
const MANA_C: Asset = asset!("/assets/mana/C.svg");
const MANA_X: Asset = asset!("/assets/mana/X.svg");
const MANA_S: Asset = asset!("/assets/mana/S.svg");

// Hybrid mana
const MANA_WU: Asset = asset!("/assets/mana/WU.svg");
const MANA_WB: Asset = asset!("/assets/mana/WB.svg");
const MANA_UR: Asset = asset!("/assets/mana/UR.svg");
const MANA_UB: Asset = asset!("/assets/mana/UB.svg");
const MANA_BR: Asset = asset!("/assets/mana/BR.svg");
const MANA_BG: Asset = asset!("/assets/mana/BG.svg");
const MANA_RG: Asset = asset!("/assets/mana/RG.svg");
const MANA_RW: Asset = asset!("/assets/mana/RW.svg");
const MANA_GW: Asset = asset!("/assets/mana/GW.svg");
const MANA_GU: Asset = asset!("/assets/mana/GU.svg");

// Phyrexian mana
const MANA_WP: Asset = asset!("/assets/mana/WP.svg");
const MANA_UP: Asset = asset!("/assets/mana/UP.svg");
const MANA_BP: Asset = asset!("/assets/mana/BP.svg");
const MANA_RP: Asset = asset!("/assets/mana/RP.svg");
const MANA_GP: Asset = asset!("/assets/mana/GP.svg");

// Hybrid Phyrexian mana
const MANA_WUP: Asset = asset!("/assets/mana/WUP.svg");
const MANA_WBP: Asset = asset!("/assets/mana/WBP.svg");
const MANA_URP: Asset = asset!("/assets/mana/URP.svg");
const MANA_UBP: Asset = asset!("/assets/mana/UBP.svg");
const MANA_BRP: Asset = asset!("/assets/mana/BRP.svg");
const MANA_BGP: Asset = asset!("/assets/mana/BGP.svg");
const MANA_RGP: Asset = asset!("/assets/mana/RGP.svg");
const MANA_RWP: Asset = asset!("/assets/mana/RWP.svg");
const MANA_GWP: Asset = asset!("/assets/mana/GWP.svg");
const MANA_GUP: Asset = asset!("/assets/mana/GUP.svg");

// Hybrid costs
const MANA_2W: Asset = asset!("/assets/mana/2W.svg");
const MANA_2U: Asset = asset!("/assets/mana/2U.svg");
const MANA_2B: Asset = asset!("/assets/mana/2B.svg");
const MANA_2R: Asset = asset!("/assets/mana/2R.svg");
const MANA_2G: Asset = asset!("/assets/mana/2G.svg");

// Colorless hybrid
const MANA_CW: Asset = asset!("/assets/mana/CW.svg");
const MANA_CU: Asset = asset!("/assets/mana/CU.svg");
const MANA_CB: Asset = asset!("/assets/mana/CB.svg");
const MANA_CR: Asset = asset!("/assets/mana/CR.svg");
const MANA_CG: Asset = asset!("/assets/mana/CG.svg");

fn get_mana_asset(symbol: CostSymbol) -> Option<Asset> {
    match symbol {
        CostSymbol::Generic { n } => match n {
            0 => Some(MANA_0),
            1 => Some(MANA_1),
            2 => Some(MANA_2),
            3 => Some(MANA_3),
            4 => Some(MANA_4),
            5 => Some(MANA_5),
            6 => Some(MANA_6),
            7 => Some(MANA_7),
            8 => Some(MANA_8),
            9 => Some(MANA_9),
            10 => Some(MANA_10),
            11 => Some(MANA_11),
            12 => Some(MANA_12),
            13 => Some(MANA_13),
            14 => Some(MANA_14),
            15 => Some(MANA_15),
            16 => Some(MANA_16),
            17 => Some(MANA_17),
            18 => Some(MANA_18),
            19 => Some(MANA_19),
            20 => Some(MANA_20),
            _ => None, // For numbers > 20, no assets available
        },
        CostSymbol::Color { color } => match color {
            Color::White => Some(MANA_W),
            Color::Blue => Some(MANA_U),
            Color::Black => Some(MANA_B),
            Color::Red => Some(MANA_R),
            Color::Green => Some(MANA_G),
        },
        CostSymbol::Phyrexian { color } => match color {
            Color::White => Some(MANA_WP),
            Color::Blue => Some(MANA_UP),
            Color::Black => Some(MANA_BP),
            Color::Red => Some(MANA_RP),
            Color::Green => Some(MANA_GP),
        },
        CostSymbol::PhyrexianFuse { color1, color2 } => match (color1, color2) {
            (Color::White, Color::Blue) => Some(MANA_WUP),
            (Color::White, Color::Black) => Some(MANA_WBP),
            (Color::Blue, Color::Red) => Some(MANA_URP),
            (Color::Blue, Color::Black) => Some(MANA_UBP),
            (Color::Black, Color::Red) => Some(MANA_BRP),
            (Color::Black, Color::Green) => Some(MANA_BGP),
            (Color::Red, Color::Green) => Some(MANA_RGP),
            (Color::Red, Color::White) => Some(MANA_RWP),
            (Color::Green, Color::White) => Some(MANA_GWP),
            (Color::Green, Color::Blue) => Some(MANA_GUP),
            _ => None, // Shouldn't happen with standard color pairs
        },
        CostSymbol::Fuse { color1, color2 } => match (color1, color2) {
            (Color::White, Color::Blue) => Some(MANA_WU),
            (Color::White, Color::Black) => Some(MANA_WB),
            (Color::Blue, Color::Red) => Some(MANA_UR),
            (Color::Blue, Color::Black) => Some(MANA_UB),
            (Color::Black, Color::Red) => Some(MANA_BR),
            (Color::Black, Color::Green) => Some(MANA_BG),
            (Color::Red, Color::Green) => Some(MANA_RG),
            (Color::Red, Color::White) => Some(MANA_RW),
            (Color::Green, Color::White) => Some(MANA_GW),
            (Color::Green, Color::Blue) => Some(MANA_GU),
            _ => None, // Shouldn't happen with standard color pairs
        },
        CostSymbol::Variable => Some(MANA_X),
        CostSymbol::Snow => Some(MANA_S),
        CostSymbol::Colorless => Some(MANA_C),
        CostSymbol::ColorlessHybrid { color } => match color {
            Color::White => Some(MANA_CW),
            Color::Blue => Some(MANA_CU),
            Color::Black => Some(MANA_CB),
            Color::Red => Some(MANA_CR),
            Color::Green => Some(MANA_CG),
        },
        CostSymbol::TwoBird { color } => match color {
            Color::White => Some(MANA_2W),
            Color::Blue => Some(MANA_2U),
            Color::Black => Some(MANA_2B),
            Color::Red => Some(MANA_2R),
            Color::Green => Some(MANA_2G),
        },
    }
}

#[component]
pub fn ManaCost(cost: Cost) -> Element {
    rsx! {
        div { class: "flex items-center",
            for symbol in cost {
                if let Some(asset) = get_mana_asset(symbol) {
                    img {
                        src: asset,
                        alt: "{symbol}",
                        class: "w-4 h-4 flex-shrink-0",
                        style: "object-fit: contain; display: block;"
                    }
                } else {
                    // Fallback for missing SVGs
                    div {
                        class: "w-4 h-4 bg-gray-200 rounded-full flex items-center justify-center text-xs flex-shrink-0",
                        "{symbol}"
                    }
                }
            }
        }
    }
}
