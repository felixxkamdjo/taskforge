use chrono::{DateTime, Utc, Datelike, Timelike, Duration};
use crate::types::{Schedule, CronField};

/// Calcule la prochaine occurrence d'une planification à partir de "from"
pub fn next_occurrence(schedule: &Schedule, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match schedule {
        Schedule::Cron { minute, hour, day_of_month, month, day_of_week } => {
            next_cron_occurrence(minute, hour, day_of_month, month, day_of_week, from)
        }
        Schedule::Daily => next_daily_occurrence(from),
        Schedule::Hourly => next_hourly_occurrence(from),
        Schedule::Weekly => next_weekly_occurrence(from),
        Schedule::Monthly => next_monthly_occurrence(from),
        Schedule::Yearly => next_yearly_occurrence(from),
        Schedule::EveryMinutes(minutes) => next_every_minutes_occurrence(*minutes, from),
    }
}

fn next_cron_occurrence(
    minute: &CronField,
    hour: &CronField,
    day_of_month: &CronField,
    month: &CronField,
    day_of_week: &CronField,
    from: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let mut current = from + Duration::minutes(1);
    let max_iterations = 525600;
    
    for _ in 0..max_iterations {
        if matches_field(current.minute(), minute)
            && matches_field(current.hour(), hour)
            && matches_field(current.day(), day_of_month)
            && matches_field(current.month(), month)
            && matches_weekday(current.date_naive().weekday().num_days_from_sunday(), day_of_week)
        {
            return Some(current);
        }
        current = current + Duration::minutes(1);
    }
    
    None
}

fn next_daily_occurrence(from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let tomorrow = from.date_naive() + Duration::days(1);
    Some(tomorrow.and_hms_opt(0, 0, 0)?.and_utc())
}

fn next_hourly_occurrence(from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let next_hour = from + Duration::hours(1);
    Some(next_hour.date_naive().and_hms_opt(next_hour.hour(), 0, 0)?.and_utc())
}

fn next_weekly_occurrence(from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    // Prochain lundi à 00:00
    let current_weekday = from.date_naive().weekday().num_days_from_monday();
    
    let days_until_monday = if current_weekday == 0 {
        7 // On est lundi → prochain lundi dans 7 jours
    } else {
        7 - current_weekday
    };
    
    Some((from.date_naive() + Duration::days(days_until_monday as i64))
        .and_hms_opt(0, 0, 0)?
        .and_utc())
}

fn next_monthly_occurrence(from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let (year, month) = if from.month() == 12 {
        (from.year() + 1, 1)
    } else {
        (from.year(), from.month() + 1)
    };
    Some(chrono::NaiveDate::from_ymd_opt(year, month, 1)?
        .and_hms_opt(0, 0, 0)?
        .and_utc())
}

fn next_yearly_occurrence(from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    Some(chrono::NaiveDate::from_ymd_opt(from.year() + 1, 1, 1)?
        .and_hms_opt(0, 0, 0)?
        .and_utc())
}

fn next_every_minutes_occurrence(minutes: u32, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    Some(from + Duration::minutes(minutes as i64))
}

fn matches_field(value: u32, field: &CronField) -> bool {
    match field {
        CronField::Any => true,
        CronField::Single(v) => value == *v,
        CronField::List(values) => values.contains(&value),
        CronField::Range(start, end) => value >= *start && value <= *end,
        CronField::Step(base, step) => {
            if *base == 0 {
                value % step == 0
            } else {
                (value - *base) % *step == 0
            }
        }
    }
}

fn matches_weekday(weekday_num: u32, field: &CronField) -> bool {
    match field {
        CronField::Any => true,
        CronField::Single(v) => {
            if *v == 7 {
                weekday_num == 0
            } else {
                weekday_num == *v
            }
        }
        CronField::List(values) => {
            values.iter().any(|v| {
                if *v == 7 {
                    weekday_num == 0
                } else {
                    weekday_num == *v
                }
            })
        }
        CronField::Range(start, end) => weekday_num >= *start && weekday_num <= *end,
        CronField::Step(_, _) => true,
    }
}