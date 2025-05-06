use std::{fmt::Display, str::FromStr};

use regex::Regex;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum Color {
    White,
    Blue,
    Black,
    Red,
    Green,
}

impl FromStr for Color {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "W" => Ok(Color::White),
            "U" => Ok(Color::Blue),
            "B" => Ok(Color::Black),
            "R" => Ok(Color::Red),
            "G" => Ok(Color::Green),
            _ => Err(format!("Unknown color: {s}")),
        }
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Color::White => "W",
                Color::Blue => "U",
                Color::Black => "B",
                Color::Red => "R",
                Color::Green => "G",
            }
        )
    }
}

pub enum CostSymbol {
    Colorless { n: u8 },
    Color { color: Color },
    Phyrexian { color: Color },
    Fuse { color1: Color, color2: Color },
    Variable,
    Snow,
}

impl Display for CostSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        match self {
            CostSymbol::Colorless { n } => write!(f, "{n}"),
            CostSymbol::Color { color } => write!(f, "{color}"),
            CostSymbol::Phyrexian { color } => write!(f, "{color}/P"),
            CostSymbol::Fuse { color1, color2 } => write!(f, "{color1}/{color2}"),
            CostSymbol::Variable => write!(f, "X"),
            CostSymbol::Snow => write!(f, "S"),
        }?;
        write!(f, "}}")
    }
}

impl FromStr for CostSymbol {
    type Err = String;
    fn from_str(s: &str) -> Result<CostSymbol, String> {
        match s {
            // Variable costs
            "X" => Ok(CostSymbol::Variable),

            // Snow mana
            "S" => Ok(CostSymbol::Snow),

            // Colorless mana
            s if s.chars().all(|c| c.is_ascii_digit()) => match s.parse::<u8>() {
                Ok(n) => Ok(CostSymbol::Colorless { n }),
                Err(_) => Err(format!("Invalid colorless mana symbol: {s}")),
            },

            // Phyrexian mana
            "W/P" => Ok(CostSymbol::Phyrexian {
                color: Color::White,
            }),
            "U/P" => Ok(CostSymbol::Phyrexian { color: Color::Blue }),
            "B/P" => Ok(CostSymbol::Phyrexian {
                color: Color::Black,
            }),
            "R/P" => Ok(CostSymbol::Phyrexian { color: Color::Red }),
            "G/P" => Ok(CostSymbol::Phyrexian {
                color: Color::Green,
            }),

            // Hybrid/fuse mana
            s if s.contains('/') && !s.contains("/P") => {
                let parts: Vec<&str> = s.split('/').collect();
                if parts.len() != 2 {
                    return Err(format!("Invalid hybrid mana symbol: {s}"));
                }

                Ok(CostSymbol::Fuse {
                    color1: parts[0].parse()?,
                    color2: parts[1].parse()?,
                })
            }

            s => Ok(CostSymbol::Color { color: s.parse()? }),
        }
    }
}

pub struct Cost {
    inner: Vec<CostSymbol>,
}

impl FromStr for Cost {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Cost { inner: Vec::new() });
        }

        // Use regex to find all patterns like "{X}" where X is any sequence of chars
        let mut symbols = Vec::new();

        // Extract and parse each symbol
        let re = Regex::new(r"\{([^{}]+)\}").expect("invalid regex");

        for cap in re.captures_iter(s) {
            let symbol_str = &cap[1]; // Extract what's inside the braces
            symbols.push(symbol_str.parse()?);
        }

        if symbols.is_empty() {
            return Err(format!("Invalid mana cost: {s}"));
        }

        Ok(Cost { inner: symbols })

        // while let Some(c) = chars.next() {
        //     match c {
        //         '{' => {
        //             let mut symbol_str = String::new();

        //             // Read until we find a closing brace
        //             while let Some(&next) = chars.peek() {
        //                 if next == '}' {
        //                     // Parse the symbol and add it to our list
        //                     symbols.push(symbol_str.parse()?);
        //                     break;
        //                 }

        //                 chars.next(); // Consume the character
        //                 symbol_str.push(next);
        //             }

        //             if !symbol_str.is_empty() && chars.peek().is_none() {
        //                 return Err(format!(
        //                     "Cost symbol missing close parentheses: {symbol_str}"
        //                 ));
        //             }
        //         }
        //         _ => return Err(format!("Expected '{{' at start of symbol, got '{c}'")),
        //     }
        // }

        // Ok(Cost { inner: symbols })
    }
}

impl Display for Cost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.iter().try_fold((), |(), c| c.fmt(f))
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_parse_simple_costs() {
        assert_eq!(
            "{1}",
            Cost::from_str("{1}")
                .expect("Failed to parse {1}")
                .to_string()
        );
        assert_eq!(
            "{W}",
            Cost::from_str("{W}")
                .expect("Failed to parse {W}")
                .to_string()
        );
        assert_eq!(
            "{U}",
            Cost::from_str("{U}")
                .expect("Failed to parse {U}")
                .to_string()
        );
        assert_eq!(
            "{B}",
            Cost::from_str("{B}")
                .expect("Failed to parse {B}")
                .to_string()
        );
        assert_eq!(
            "{R}",
            Cost::from_str("{R}")
                .expect("Failed to parse {R}")
                .to_string()
        );
        assert_eq!(
            "{G}",
            Cost::from_str("{G}")
                .expect("Failed to parse {G}")
                .to_string()
        );
    }

    #[test]
    fn test_parse_complex_costs() {
        // Test a complex mana cost: {2}{W}{U}
        let cost = Cost::from_str("{2}{W}{U}").expect("Failed to parse {2}{W}{U}");
        assert_eq!("{2}{W}{U}", cost.to_string());

        // Test a cost with phyrexian mana: {1}{W/P}{R}
        let cost = Cost::from_str("{1}{W/P}{R}").expect("Failed to parse {1}{W/P}{R}");
        assert_eq!("{1}{W/P}{R}", cost.to_string());

        // Test hybrid mana: {W/U}
        let cost = Cost::from_str("{W/U}").expect("Failed to parse {W/U}");
        assert_eq!("{W/U}", cost.to_string());
    }

    #[test]
    fn test_parse_variable_and_special_costs() {
        // Test X cost
        let cost = Cost::from_str("{X}{R}{R}").expect("Failed to parse {X}{R}{R}");
        assert_eq!("{X}{R}{R}", cost.to_string());

        // Test snow mana
        let cost = Cost::from_str("{S}{G}").expect("Failed to parse {S}{G}");
        assert_eq!("{S}{G}", cost.to_string());

        // Test a mix of different symbols
        let cost = Cost::from_str("{X}{2}{W/U}{B/P}").expect("Failed to parse {X}{2}{W/U}{B/P}");
        assert_eq!("{X}{2}{W/U}{B/P}", cost.to_string());
    }

    #[test]
    fn test_parse_colorless_costs() {
        // Test various colorless amounts
        assert_eq!(
            "{0}",
            Cost::from_str("{0}")
                .expect("Failed to parse {0}")
                .to_string()
        );
        assert_eq!(
            "{1}",
            Cost::from_str("{1}")
                .expect("Failed to parse {1}")
                .to_string()
        );
        assert_eq!(
            "{10}",
            Cost::from_str("{10}")
                .expect("Failed to parse {10}")
                .to_string()
        );
        assert_eq!(
            "{15}",
            Cost::from_str("{15}")
                .expect("Failed to parse {15}")
                .to_string()
        );
    }

    #[test]
    fn test_parse_all_phyrexian_costs() {
        assert_eq!(
            "{W/P}",
            Cost::from_str("{W/P}")
                .expect("Failed to parse {W/P}")
                .to_string()
        );
        assert_eq!(
            "{U/P}",
            Cost::from_str("{U/P}")
                .expect("Failed to parse {U/P}")
                .to_string()
        );
        assert_eq!(
            "{B/P}",
            Cost::from_str("{B/P}")
                .expect("Failed to parse {B/P}")
                .to_string()
        );
        assert_eq!(
            "{R/P}",
            Cost::from_str("{R/P}")
                .expect("Failed to parse {R/P}")
                .to_string()
        );
        assert_eq!(
            "{G/P}",
            Cost::from_str("{G/P}")
                .expect("Failed to parse {G/P}")
                .to_string()
        );
    }

    #[test]
    fn test_parse_all_hybrid_costs() {
        // Test all color combinations
        let pairs = [
            ("W/U", "W/U"),
            ("W/B", "W/B"),
            ("W/R", "W/R"),
            ("W/G", "W/G"),
            ("U/B", "U/B"),
            ("U/R", "U/R"),
            ("U/G", "U/G"),
            ("B/R", "B/R"),
            ("B/G", "B/G"),
            ("R/G", "R/G"),
        ];

        for (input, expected) in pairs {
            let cost_str = format!("{{{input}}}");
            let expected_str = format!("{{{expected}}}");
            assert_eq!(
                expected_str,
                Cost::from_str(&cost_str)
                    .unwrap_or_else(|_| panic!("Failed to parse {cost_str}"))
                    .to_string()
            );
        }
    }

    #[test]
    fn test_empty_cost() {
        // Test empty cost (a card with no mana cost)
        let cost = Cost::from_str("").expect("Failed to parse empty cost");
        assert_eq!("", cost.to_string());
    }

    #[test]
    fn test_invalid_costs() {
        // Missing closing brace
        assert!(Cost::from_str("{W").is_err());

        // Invalid symbols
        assert!(Cost::from_str("{Z}").is_err());
        assert!(Cost::from_str("{WW}").is_err());

        // Malformed inputs
        assert!(Cost::from_str("W").is_err());

        // Invalid hybrid mana
        assert!(Cost::from_str("{W/Z}").is_err());
        assert!(Cost::from_str("{W/U/B}").is_err());
    }

    #[test]
    fn test_real_card_costs() {
        // Test some real card mana costs

        // Black Lotus: {0}
        assert_eq!(
            "{0}",
            Cost::from_str("{0}")
                .expect("Failed to parse Black Lotus cost {0}")
                .to_string()
        );

        // Lightning Bolt: {R}
        assert_eq!(
            "{R}",
            Cost::from_str("{R}")
                .expect("Failed to parse Lightning Bolt cost {R}")
                .to_string()
        );

        // Counterspell: {U}{U}
        assert_eq!(
            "{U}{U}",
            Cost::from_str("{U}{U}")
                .expect("Failed to parse Counterspell cost {U}{U}")
                .to_string()
        );

        // Wrath of God: {2}{W}{W}
        assert_eq!(
            "{2}{W}{W}",
            Cost::from_str("{2}{W}{W}")
                .expect("Failed to parse Wrath of God cost {2}{W}{W}")
                .to_string()
        );

        // Nicol Bolas, Dragon-God: {U}{B}{B}{R}
        assert_eq!(
            "{U}{B}{B}{R}",
            Cost::from_str("{U}{B}{B}{R}")
                .expect("Failed to parse Nicol Bolas cost {U}{B}{B}{R}")
                .to_string()
        );

        // Sphinx of the Steel Wind: {5}{W}{U}{B}
        assert_eq!(
            "{5}{W}{U}{B}",
            Cost::from_str("{5}{W}{U}{B}")
                .expect("Failed to parse Sphinx cost {5}{W}{U}{B}")
                .to_string()
        );
    }

    #[test]
    fn test_round_trip_parsing() {
        let test_costs = [
            "{1}{W}{U}",
            "{X}{X}{R}",
            "{0}",
            "{15}",
            "{W/U}{W/U}{W/U}",
            "{W/P}{R}{G}",
            "{S}{S}{1}",
            "{X}{Y}", // This should fail if Y is not a valid symbol
        ];

        for cost_str in &test_costs {
            // Skip the invalid test case
            if *cost_str == "{X}{Y}" {
                continue;
            }

            let cost = match Cost::from_str(cost_str) {
                Ok(c) => c,
                Err(e) => panic!("Failed to parse valid cost {cost_str}: {e}"),
            };

            let serialized = cost.to_string();
            assert_eq!(
                *cost_str, serialized,
                "Round-trip parsing failed for {cost_str}"
            );

            // Parse it again to ensure we can re-parse our output
            let reparsed = Cost::from_str(&serialized)
                .unwrap_or_else(|_| panic!("Failed to re-parse serialized cost: {serialized}"));
            assert_eq!(
                serialized,
                reparsed.to_string(),
                "Re-parsing failed for {serialized}"
            );
        }
    }
}
