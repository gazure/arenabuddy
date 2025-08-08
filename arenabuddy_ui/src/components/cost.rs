use arenabuddy_core::models::Cost;
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
const MANA_INVERTED_S: Asset = asset!("/assets/mana/InvertedS.svg");

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

fn get_mana_asset(filename: &str) -> Option<Asset> {
    match filename {
        "0.svg" => Some(MANA_0),
        "1.svg" => Some(MANA_1),
        "2.svg" => Some(MANA_2),
        "3.svg" => Some(MANA_3),
        "4.svg" => Some(MANA_4),
        "5.svg" => Some(MANA_5),
        "6.svg" => Some(MANA_6),
        "7.svg" => Some(MANA_7),
        "8.svg" => Some(MANA_8),
        "9.svg" => Some(MANA_9),
        "10.svg" => Some(MANA_10),
        "11.svg" => Some(MANA_11),
        "12.svg" => Some(MANA_12),
        "13.svg" => Some(MANA_13),
        "14.svg" => Some(MANA_14),
        "15.svg" => Some(MANA_15),
        "16.svg" => Some(MANA_16),
        "17.svg" => Some(MANA_17),
        "18.svg" => Some(MANA_18),
        "19.svg" => Some(MANA_19),
        "20.svg" => Some(MANA_20),
        "W.svg" => Some(MANA_W),
        "U.svg" => Some(MANA_U),
        "B.svg" => Some(MANA_B),
        "R.svg" => Some(MANA_R),
        "G.svg" => Some(MANA_G),
        "C.svg" => Some(MANA_C),
        "X.svg" => Some(MANA_X),
        "S.svg" => Some(MANA_S),
        "InvertedS.svg" => Some(MANA_INVERTED_S),
        "WU.svg" => Some(MANA_WU),
        "WB.svg" => Some(MANA_WB),
        "UR.svg" => Some(MANA_UR),
        "UB.svg" => Some(MANA_UB),
        "BR.svg" => Some(MANA_BR),
        "BG.svg" => Some(MANA_BG),
        "RG.svg" => Some(MANA_RG),
        "RW.svg" => Some(MANA_RW),
        "GW.svg" => Some(MANA_GW),
        "GU.svg" => Some(MANA_GU),
        "WP.svg" => Some(MANA_WP),
        "UP.svg" => Some(MANA_UP),
        "BP.svg" => Some(MANA_BP),
        "RP.svg" => Some(MANA_RP),
        "GP.svg" => Some(MANA_GP),
        "WUP.svg" => Some(MANA_WUP),
        "WBP.svg" => Some(MANA_WBP),
        "URP.svg" => Some(MANA_URP),
        "UBP.svg" => Some(MANA_UBP),
        "BRP.svg" => Some(MANA_BRP),
        "BGP.svg" => Some(MANA_BGP),
        "RGP.svg" => Some(MANA_RGP),
        "RWP.svg" => Some(MANA_RWP),
        "GWP.svg" => Some(MANA_GWP),
        "GUP.svg" => Some(MANA_GUP),
        "2W.svg" => Some(MANA_2W),
        "2U.svg" => Some(MANA_2U),
        "2B.svg" => Some(MANA_2B),
        "2R.svg" => Some(MANA_2R),
        "2G.svg" => Some(MANA_2G),
        "CW.svg" => Some(MANA_CW),
        "CU.svg" => Some(MANA_CU),
        "CB.svg" => Some(MANA_CB),
        "CR.svg" => Some(MANA_CR),
        "CG.svg" => Some(MANA_CG),
        _ => None,
    }
}

#[component]
pub fn ManaCost(cost: Cost) -> Element {
    rsx! {
        div { class: "flex items-center",
            for symbol in cost {
                if let Some(asset) = get_mana_asset(symbol.svg_file()) {
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