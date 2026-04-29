/// Tests d'intégration du module registry (P2).
/// Vérifie load_config + TaskRegistry comme consommé par P3.
use taskforge::registry::{load_config, TaskRegistry};
use taskforge::types::Schedule;
use chrono::{Utc, TimeZone};
use std::io::Write;
use tempfile::NamedTempFile;

fn write_toml(content: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{}", content).unwrap();
    f
}

// ---------------------------------------------------------------------------
// load_config
// ---------------------------------------------------------------------------

#[test]
fn test_load_and_count_tasks() {
    let toml = r#"
[[task]]
id = "t1"
name = "Tâche 1"
command = "echo 1"
schedule = "@daily"

[[task]]
id = "t2"
name = "Tâche 2"
command = "echo 2"
schedule = "*/15 * * * *"
"#;

    let f = write_toml(toml);
    let tasks = load_config(f.path()).unwrap();
    assert_eq!(tasks.len(), 2);
}

#[test]
fn test_load_schedule_cron_parsed_correctly() {
    let toml = r#"
[[task]]
id = "backup"
name = "Backup"
command = "pg_dump db"
schedule = "0 1 * * *"
"#;

    let f = write_toml(toml);
    let tasks = load_config(f.path()).unwrap();

    // doit être interprété comme cron et non macro
    assert!(matches!(tasks[0].schedule, Schedule::Cron { .. }));
}

#[test]
fn test_load_macro_daily() {
    let toml = r#"
[[task]]
id = "clean"
name = "Nettoyage"
command = "rm -rf /tmp/*"
schedule = "@daily"
"#;

    let f = write_toml(toml);
    let tasks = load_config(f.path()).unwrap();
    assert_eq!(tasks[0].schedule, Schedule::Daily);
}

#[test]
fn test_load_macro_every() {
    let toml = r#"
[[task]]
id = "uptime"
name = "Uptime"
command = "systemctl status nginx"
schedule = "@every 5m"
"#;

    let f = write_toml(toml);
    let tasks = load_config(f.path()).unwrap();
    assert_eq!(tasks[0].schedule, Schedule::EveryMinutes(5));
}

#[test]
fn test_load_defaults_applied() {
    let toml = r#"
[[task]]
id = "minimal"
name = "Minimal"
command = "true"
schedule = "@hourly"
"#;

    let f = write_toml(toml);
    let tasks = load_config(f.path()).unwrap();

    assert_eq!(tasks[0].timeout_seconds, 60);
    assert_eq!(tasks[0].max_retries, 0);
    assert!(tasks[0].enabled);
}

#[test]
fn test_load_disabled_task() {
    let toml = r#"
[[task]]
id = "disabled"
name = "Désactivée"
command = "echo off"
schedule = "@weekly"
enabled = false
"#;

    let f = write_toml(toml);
    let tasks = load_config(f.path()).unwrap();
    assert!(!tasks[0].enabled);
}

#[test]
fn test_load_invalid_schedule_is_error() {
    let toml = r#"
[[task]]
id = "bad"
name = "Mauvaise"
command = "echo bad"
schedule = "pas_une_expression_cron"
"#;

    let f = write_toml(toml);
    assert!(load_config(f.path()).is_err());
}

#[test]
fn test_load_duplicate_id_is_error() {
    let toml = r#"
[[task]]
id = "dup"
name = "A"
command = "echo a"
schedule = "@daily"

[[task]]
id = "dup"
name = "B"
command = "echo b"
schedule = "@hourly"
"#;

    let f = write_toml(toml);
    assert!(load_config(f.path()).is_err());
}

// ---------------------------------------------------------------------------
// TaskRegistry (usage type P3)
// ---------------------------------------------------------------------------

#[test]
fn test_registry_from_config_file() {
    let toml = r#"
[[task]]
id = "t1"
name = "T1"
command = "echo t1"
schedule = "@daily"

[[task]]
id = "t2"
name = "T2"
command = "echo t2"
schedule = "@hourly"
enabled = false
"#;

    let f = write_toml(toml);
    let tasks = load_config(f.path()).unwrap();
    let reg = TaskRegistry::from_tasks(tasks);

    assert_eq!(reg.len(), 2);
    assert_eq!(reg.active_count(), 1); // t2 désactivée
}

#[test]
fn test_registry_set_enabled_then_due() {
    let toml = r#"
[[task]]
id = "t1"
name = "T1"
command = "echo t1"
schedule = "@every 5m"
enabled = false
"#;

    let f = write_toml(toml);
    let tasks = load_config(f.path()).unwrap();
    let mut reg = TaskRegistry::from_tasks(tasks);

    let from = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

    assert!(reg.due_tasks(from, 600).is_empty()); // désactivée

    reg.set_enabled("t1", true);
    assert!(!reg.due_tasks(from, 600).is_empty()); // activée
}

#[test]
fn test_registry_upcoming_order() {
    let toml = r#"
[[task]]
id = "slow"
name = "Lente"
command = "echo slow"
schedule = "@daily"

[[task]]
id = "fast"
name = "Rapide"
command = "echo fast"
schedule = "@every 5m"
"#;

    let f = write_toml(toml);
    let tasks = load_config(f.path()).unwrap();
    let reg = TaskRegistry::from_tasks(tasks);

    let from = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
    let upcoming = reg.upcoming(from);

    assert_eq!(upcoming.len(), 2);
    assert_eq!(upcoming[0].0.id, "fast"); // plus proche en premier
    assert_eq!(upcoming[1].0.id, "slow");
}

#[test]
fn test_registry_unregister_task() {
    let toml = r#"
[[task]]
id = "tmp"
name = "Temporaire"
command = "echo tmp"
schedule = "@hourly"
"#;

    let f = write_toml(toml);
    let tasks = load_config(f.path()).unwrap();
    let mut reg = TaskRegistry::from_tasks(tasks);

    assert!(reg.get("tmp").is_some());

    reg.unregister("tmp");

    assert!(reg.get("tmp").is_none());
    assert!(reg.is_empty());
}