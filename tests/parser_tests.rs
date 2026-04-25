use taskforge::parser::{parse_cron_expression, parse_macro, next_occurrence};
use taskforge::types::{Schedule, CronField};
use chrono::{Utc, TimeZone, Datelike, Timelike};

// Tests de parsing d'expressions cron
#[test]
fn test_parse_simple_cron() {
    let result = parse_cron_expression("30 2 * * 1");
    assert!(result.is_ok());
}

#[test]
fn test_parse_cron_with_list() {
    let result = parse_cron_expression("0,30 9,17 * * 1-5");
    assert!(result.is_ok());
}

#[test]
fn test_parse_cron_with_step() {
    let result = parse_cron_expression("*/15 * * * *");
    assert!(result.is_ok());
}

#[test]
fn test_parse_cron_with_range_and_step() {
    let result = parse_cron_expression("1-30/5 * * * *");
    assert!(result.is_ok());
}

#[test]
fn test_parse_cron_too_few_fields() {
    let result = parse_cron_expression("* * *");
    assert!(result.is_err());
}

#[test]
fn test_parse_cron_too_many_fields() {
    let result = parse_cron_expression("* * * * * *");
    assert!(result.is_err());
}

#[test]
fn test_parse_cron_invalid_minute() {
    let result = parse_cron_expression("60 * * * *");
    assert!(result.is_err());
}

#[test]
fn test_parse_cron_invalid_hour() {
    let result = parse_cron_expression("* 24 * * *");
    assert!(result.is_err());
}

#[test]
fn test_parse_cron_all_wildcards() {
    let result = parse_cron_expression("* * * * *");
    assert!(result.is_ok());
}

// Tests de parsing des macros
#[test]
fn test_parse_macro_daily() {
    let result = parse_macro("@daily");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Schedule::Daily);
}

#[test]
fn test_parse_macro_hourly() {
    let result = parse_macro("@hourly");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Schedule::Hourly);
}

#[test]
fn test_parse_macro_weekly() {
    let result = parse_macro("@weekly");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Schedule::Weekly);
}

#[test]
fn test_parse_macro_monthly() {
    let result = parse_macro("@monthly");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Schedule::Monthly);
}

#[test]
fn test_parse_macro_yearly() {
    let result = parse_macro("@yearly");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Schedule::Yearly);
}

#[test]
fn test_parse_macro_annually_alias() {
    let result = parse_macro("@annually");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Schedule::Yearly);
}

#[test]
fn test_parse_macro_midnight_alias() {
    let result = parse_macro("@midnight");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Schedule::Daily);
}

#[test]
fn test_parse_macro_every_5m() {
    let result = parse_macro("@every 5m");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Schedule::EveryMinutes(5));
}

#[test]
fn test_parse_macro_every_2h() {
    let result = parse_macro("@every 2h");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Schedule::EveryMinutes(120));
}

#[test]
fn test_parse_macro_every_30s() {
    let result = parse_macro("@every 30s");
    assert!(result.is_some());
}

#[test]
fn test_parse_macro_invalid() {
    let result = parse_macro("@invalid");
    assert!(result.is_none());
}

#[test]
fn test_parse_macro_no_at_sign() {
    let result = parse_macro("daily");
    assert!(result.is_none());
}

// Tests de next_occurrence
#[test]
fn test_next_occurrence_daily_at_2am() {
    let schedule = Schedule::Cron {
        minute: CronField::Single(0),
        hour: CronField::Single(2),
        day_of_month: CronField::Any,
        month: CronField::Any,
        day_of_week: CronField::Any,
    };
    
    let from = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let next = next_occurrence(&schedule, from);
    assert!(next.is_some());
    let next = next.unwrap();
    assert_eq!(next.hour(), 2);
    assert_eq!(next.minute(), 0);
    assert_eq!(next.day(), 1);
}

#[test]
fn test_next_occurrence_every_15_min() {
    let schedule = Schedule::Cron {
        minute: CronField::Step(0, 15),
        hour: CronField::Any,
        day_of_month: CronField::Any,
        month: CronField::Any,
        day_of_week: CronField::Any,
    };
    
    let from = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
    let next = next_occurrence(&schedule, from);
    assert!(next.is_some());
    let next = next.unwrap();
    assert_eq!(next.minute(), 15);
    assert_eq!(next.hour(), 12);
}

#[test]
fn test_next_daily_occurrence() {
    let from = Utc.with_ymd_and_hms(2024, 1, 1, 15, 30, 0).unwrap();
    let next = next_occurrence(&Schedule::Daily, from);
    assert!(next.is_some());
    let next = next.unwrap();
    assert_eq!(next.hour(), 0);
    assert_eq!(next.minute(), 0);
    assert_eq!(next.day(), 2);
    assert_eq!(next.month(), 1);
    assert_eq!(next.year(), 2024);
}

#[test]
fn test_next_hourly_occurrence() {
    let from = Utc.with_ymd_and_hms(2024, 1, 1, 15, 30, 0).unwrap();
    let next = next_occurrence(&Schedule::Hourly, from);
    assert!(next.is_some());
    let next = next.unwrap();
    assert_eq!(next.hour(), 16);
    assert_eq!(next.minute(), 0);
}

#[test]
fn test_next_every_5_minutes() {
    let from = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
    let next = next_occurrence(&Schedule::EveryMinutes(5), from);
    assert!(next.is_some());
    assert_eq!(next.unwrap().minute(), 5);
}

#[test]
fn test_next_occurrence_specific_time_past() {
    let schedule = parse_cron_expression("30 14 * * *").unwrap();
    let from = Utc.with_ymd_and_hms(2024, 6, 15, 14, 29, 0).unwrap();
    let next = next_occurrence(&schedule, from).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2024, 6, 15, 14, 30, 0).unwrap());
}

#[test]
fn test_next_occurrence_specific_time_future() {
    let schedule = parse_cron_expression("30 14 * * *").unwrap();
    let from = Utc.with_ymd_and_hms(2024, 6, 15, 14, 31, 0).unwrap();
    let next = next_occurrence(&schedule, from).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2024, 6, 16, 14, 30, 0).unwrap());
}

#[test]
fn test_next_occurrence_monday_cron() {
    let schedule = parse_cron_expression("0 9 * * 1").unwrap();
    let from = Utc.with_ymd_and_hms(2023, 3, 31, 10, 0, 0).unwrap();
    let next = next_occurrence(&schedule, from).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2023, 4, 3, 9, 0, 0).unwrap());
}

#[test]
fn test_next_occurrence_midnight() {
    let schedule = parse_cron_expression("0 0 * * *").unwrap();
    let from = Utc.with_ymd_and_hms(2024, 6, 15, 23, 59, 0).unwrap();
    let next = next_occurrence(&schedule, from).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2024, 6, 16, 0, 0, 0).unwrap());
}

#[test]
fn test_next_monthly_occurrence() {
    let from = Utc.with_ymd_and_hms(2024, 3, 15, 10, 0, 0).unwrap();
    let next = next_occurrence(&Schedule::Monthly, from).unwrap();
    assert_eq!(next.year(), 2024);
    assert_eq!(next.month(), 4);
    assert_eq!(next.day(), 1);
    assert_eq!(next.hour(), 0);
    assert_eq!(next.minute(), 0);
}

#[test]
fn test_next_monthly_occurrence_december() {
    let from = Utc.with_ymd_and_hms(2024, 12, 15, 10, 0, 0).unwrap();
    let next = next_occurrence(&Schedule::Monthly, from).unwrap();
    assert_eq!(next.year(), 2025);
    assert_eq!(next.month(), 1);
    assert_eq!(next.day(), 1);
}

#[test]
fn test_next_yearly_occurrence() {
    let from = Utc.with_ymd_and_hms(2024, 6, 15, 10, 0, 0).unwrap();
    let next = next_occurrence(&Schedule::Yearly, from).unwrap();
    assert_eq!(next.year(), 2025);
    assert_eq!(next.month(), 1);
    assert_eq!(next.day(), 1);
}

// Tests de Display
#[test]
fn test_display_schedule_daily() {
    assert_eq!(format!("{}", Schedule::Daily), "@daily");
}

#[test]
fn test_display_schedule_hourly() {
    assert_eq!(format!("{}", Schedule::Hourly), "@hourly");
}

#[test]
fn test_display_schedule_every_minutes() {
    assert_eq!(format!("{}", Schedule::EveryMinutes(10)), "@every 10m");
}

#[test]
fn test_display_cron_field_any() {
    assert_eq!(format!("{}", CronField::Any), "*");
}

#[test]
fn test_display_cron_field_single() {
    assert_eq!(format!("{}", CronField::Single(5)), "5");
}

#[test]
fn test_display_cron_field_list() {
    assert_eq!(format!("{}", CronField::List(vec![1, 3, 5])), "1,3,5");
}

#[test]
fn test_display_cron_field_range() {
    assert_eq!(format!("{}", CronField::Range(1, 5)), "1-5");
}

// Tests combinés parsing + next_occurrence
#[test]
fn test_parse_and_next_every_10_minutes() {
    let schedule = parse_macro("@every 10m").unwrap();
    let from = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
    let next = next_occurrence(&schedule, from).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2024, 1, 1, 12, 10, 0).unwrap());
}

#[test]
fn test_parse_and_next_weekly() {
    let schedule = parse_macro("@weekly").unwrap();
    // Mardi 2 janvier 2024
    let from = Utc.with_ymd_and_hms(2024, 1, 2, 10, 0, 0).unwrap();
    let next = next_occurrence(&schedule, from).unwrap();
    // Lundi suivant : 8 janvier 2024
    assert_eq!(next, Utc.with_ymd_and_hms(2024, 1, 8, 0, 0, 0).unwrap());
}