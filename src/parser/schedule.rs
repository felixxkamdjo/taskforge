use chrono::{DateTime, Utc, Datelike, Timelike, Duration};
use crate::types::{Schedule, CronField};

// Calcul de la prochaine occurrence d'une tâche selon son horaire
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

// Calcul de la prochaine occurrence pour une expression cron
fn next_cron_occurrence(
    minute: &CronField,
    hour: &CronField,
    day_of_month: &CronField,
    month: &CronField,
    day_of_week: &CronField,
    from: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let mut current = from + Duration::minutes(1);
    let max_iterations = 525600; // limite à ~1 an

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

    None // aucune occurrence trouvée
}

// Calcul de la prochaine occurrence pour les macros de planification
fn next_daily_occurrence(from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let tomorrow = from.date_naive() + Duration::days(1);
    Some(tomorrow.and_hms_opt(0, 0, 0)?.and_utc()) // minuit suivant
}

// Calcul de la prochaine occurrence pour les autres macros
fn next_hourly_occurrence(from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let next_hour = from + Duration::hours(1);
    Some(next_hour.date_naive().and_hms_opt(next_hour.hour(), 0, 0)?.and_utc()) // début d'heure
}

// Calcul de la prochaine occurrence pour les autres macros
fn next_weekly_occurrence(from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let current_weekday = from.date_naive().weekday().num_days_from_monday();
    let days_until_monday = if current_weekday == 0 { 7 } else { 7 - current_weekday }; // prochain lundi
    Some((from.date_naive() + Duration::days(days_until_monday as i64))
        .and_hms_opt(0, 0, 0)?
        .and_utc())
}

// Calcul de la prochaine occurrence pour les autres macros
fn next_monthly_occurrence(from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let (year, month) = if from.month() == 12 {
        (from.year() + 1, 1)
    } else {
        (from.year(), from.month() + 1)
    };
    Some(chrono::NaiveDate::from_ymd_opt(year, month, 1)?
        .and_hms_opt(0, 0, 0)?
        .and_utc()) // premier jour du mois
}

// Calcul de la prochaine occurrence pour les autres macros
fn next_yearly_occurrence(from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    Some(chrono::NaiveDate::from_ymd_opt(from.year() + 1, 1, 1)?
        .and_hms_opt(0, 0, 0)?
        .and_utc()) // 1er janvier
}

// Calcul de la prochaine occurrence pour les macros @every
fn next_every_minutes_occurrence(minutes: u32, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if minutes == 0 {
        return None; // intervalle invalide
    }
    Some(from + Duration::minutes(minutes as i64))
}

// Fonctions d'aide pour vérifier si une valeur correspond à un champ cron
fn matches_field(value: u32, field: &CronField) -> bool {
    match field {
        CronField::Any => true,
        CronField::Single(v) => value == *v,
        CronField::List(values) => values.contains(&value),
        CronField::Range(start, end) => value >= *start && value <= *end,
        CronField::Step(base, step) => {
            if *step == 0 {
                return false; // évite division par zéro
            }
            if value >= *base {
                (value - *base) % *step == 0 // progression régulière
            } else {
                false
            }
        }
    }
}

// Fonction d'aide pour vérifier si un jour de la semaine correspond à un champ cron
fn matches_weekday(weekday_num: u32, field: &CronField) -> bool {
    match field {
        CronField::Any => true,
        CronField::Single(v) => {
            let normalized = if *v == 7 { 0 } else { *v }; // 7 ≡ dimanche
            weekday_num == normalized
        }
        CronField::List(values) => {
            values.iter().any(|v| {
                let normalized = if *v == 7 { 0 } else { *v };
                weekday_num == normalized
            })
        }
        CronField::Range(start, end) => {
            let norm_start = if *start == 7 { 0 } else { *start };
            let norm_end = if *end == 7 { 0 } else { *end };
            if norm_start <= norm_end {
                weekday_num >= norm_start && weekday_num <= norm_end
            } else {
                weekday_num >= norm_start || weekday_num <= norm_end // plage circulaire
            }
        }
        CronField::Step(base, step) => {
            if *step == 0 {
                return false;
            }
            if weekday_num >= *base {
                (weekday_num - *base) % *step == 0
            } else {
                false
            }
        }
    }
}