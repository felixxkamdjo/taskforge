use crate::types::Schedule;

// Parser pour les macros de planification
pub fn parse_macro(input: &str) -> Option<Schedule> {
    if !input.starts_with('@') {
        return None; // ignore si pas une macro
    }

    let macro_str = &input[1..];

    match macro_str {
        "daily" | "midnight" => Some(Schedule::Daily),
        "hourly" => Some(Schedule::Hourly),
        "weekly" => Some(Schedule::Weekly),
        "monthly" => Some(Schedule::Monthly),
        "yearly" | "annually" => Some(Schedule::Yearly),
        _ => {
            if macro_str.starts_with("every ") {
                let parts: Vec<&str> = macro_str[6..].split_whitespace().collect();
                if parts.len() == 1 {
                    let value_str = parts[0];
                    if let Some(minutes) = parse_every_value(value_str) {
                        return Some(Schedule::EveryMinutes(minutes)); // intervalle valide
                    }
                }
            }
            None
        }
    }
}

// Parse la valeur de l'intervalle pour @every (ex: "5m", "2h", "30s")
fn parse_every_value(input: &str) -> Option<u32> {
    if input.ends_with('m') {
        input[..input.len() - 1].parse().ok() // minutes
    } else if input.ends_with('h') {
        input[..input.len() - 1].parse::<u32>().ok().map(|h| h * 60) // heures vers minutes
    } else if input.ends_with('s') {
        let secs: u32 = input[..input.len() - 1].parse().ok()?;
        if secs < 60 {
            None // sous-minute non supporté
        } else {
            Some(secs / 60) // secondes vers minutes
        }
    } else {
        input.parse().ok() // valeur brute en minutes
    }
}