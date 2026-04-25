use crate::types::Schedule;

/// Parse les macros comme @daily, @hourly, @every 5m, etc et les convertit en Schedule
pub fn parse_macro(input: &str) -> Option<Schedule> {
    if !input.starts_with('@') {
        return None;
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
                        return Some(Schedule::EveryMinutes(minutes));
                    }
                }
            }
            None
        }
    }
}

fn parse_every_value(input: &str) -> Option<u32> {
    if input.ends_with('m') {
        input[..input.len()-1].parse().ok()
    } else if input.ends_with('h') {
        input[..input.len()-1].parse::<u32>().ok().map(|h| h * 60)
    } else if input.ends_with('s') {
        input[..input.len()-1].parse::<u32>().ok().map(|s| s / 60)
    } else {
        input.parse().ok()
    }
}