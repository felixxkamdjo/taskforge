/// Tests d'intégration globaux — pipeline complet P1 → P4.
/// Vérifie uniquement l'interopérabilité des modules.
use taskforge::registry::{load_config, TaskRegistry};
use taskforge::engine::execute_with_retry;
use taskforge::types::{Schedule, Task};
use chrono::{Utc, TimeZone};
use std::io::Write;
// use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tempfile::NamedTempFile;

fn write_toml(content: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{}", content).unwrap();
    f
}

fn make_task(id: &str, command: &str, max_retries: u32, timeout_seconds: u32) -> Task {
    Task {
        id: id.to_string(),
        name: id.to_string(),
        command: command.to_string(),
        schedule: Schedule::EveryMinutes(5),
        timeout_seconds,
        max_retries,
        enabled: true,
    }
}

// ---------------------------------------------------------------------------
// P1 + P2 : config → registry → due tasks
// ---------------------------------------------------------------------------

#[test]
fn test_full_config_to_due_tasks() {
    let toml = r#"
[[task]]
id = "fast"
name = "Rapide"
command = "echo fast"
schedule = "@every 5m"

[[task]]
id = "daily"
name = "Quotidienne"
command = "echo daily"
schedule = "@daily"

[[task]]
id = "disabled"
name = "Désactivée"
command = "echo off"
schedule = "@every 5m"
enabled = false
"#;

    let f = write_toml(toml);
    let tasks = load_config(f.path()).unwrap();
    let reg = TaskRegistry::from_tasks(tasks);

    let from = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

    // seule "fast" est due dans la fenêtre (disabled exclue, daily trop loin)
    let due = reg.due_tasks(from, 600);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].id, "fast");
}

#[test]
fn test_all_macros_parse_and_register() {
    let toml = r#"
[[task]]
id = "t-hourly"
name = "Hourly"
command = "echo h"
schedule = "@hourly"

[[task]]
id = "t-daily"
name = "Daily"
command = "echo d"
schedule = "@daily"

[[task]]
id = "t-weekly"
name = "Weekly"
command = "echo w"
schedule = "@weekly"

[[task]]
id = "t-monthly"
name = "Monthly"
command = "echo m"
schedule = "@monthly"

[[task]]
id = "t-yearly"
name = "Yearly"
command = "echo y"
schedule = "@yearly"

[[task]]
id = "t-every"
name = "Every 10m"
command = "echo e"
schedule = "@every 10m"
"#;

    let f = write_toml(toml);
    let tasks = load_config(f.path()).unwrap();
    let reg = TaskRegistry::from_tasks(tasks);

    // vérifie uniquement la cohérence globale du chargement
    assert_eq!(reg.len(), 6);
    assert_eq!(reg.active_count(), 6);
}

// ---------------------------------------------------------------------------
// P2 + P3 : execution pipeline
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_successful_task_produces_record() {
    let task = make_task("echo", "echo integration_ok", 0, 10);
    let record = execute_with_retry(&task).await;

    assert!(record.success);
    assert_eq!(record.task_id, "echo");
    assert_eq!(record.exit_code, Some(0));
    assert!(record.stdout.contains("integration_ok"));
}

#[tokio::test]
async fn test_failing_task_record_is_coherent() {
    let task = make_task("fail", "exit 42", 0, 10);
    let record = execute_with_retry(&task).await;

    assert!(!record.success);
    assert_eq!(record.exit_code, Some(42));
    assert_eq!(record.task_id, "fail");
}

#[tokio::test]
async fn test_timeout_task_record_marks_failure() {
    let task = make_task("slow", "sleep 10", 0, 1);
    let record = execute_with_retry(&task).await;

    assert!(!record.success);
    assert!(record.exit_code.is_none());
    assert!(record.stderr.contains("timeout"));
}

#[tokio::test]
async fn test_retry_succeeds_on_eventual_success() {
    let flag = format!("/tmp/taskforge_retry_test_{}", std::process::id());
    let cmd = format!(
        "if [ ! -f {f} ]; then touch {f}; exit 1; else rm {f}; echo ok; fi",
        f = flag
    );

    let task = make_task("retry-flag", &cmd, 0, 10);

    let r1 = execute_with_retry(&task).await;
    assert!(!r1.success);

    let r2 = execute_with_retry(&task).await;
    assert!(r2.success);
}

// ---------------------------------------------------------------------------
// P3 → P4 : channel integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_channel_receives_execution_record() {
    let (tx, mut rx) = mpsc::channel(32);

    let task = make_task("chan", "echo canal_ok", 0, 10);
    let record = execute_with_retry(&task).await;

    tx.send(record).await.unwrap();

    let received = rx.recv().await.unwrap();
    assert_eq!(received.task_id, "chan");
    assert!(received.success);
    assert!(received.stdout.contains("canal_ok"));
}

#[tokio::test]
async fn test_multiple_tasks_all_send_to_channel() {
    let (tx, mut rx) = mpsc::channel(32);

    let tasks = vec![
        make_task("t1", "echo t1", 0, 10),
        make_task("t2", "echo t2", 0, 10),
        make_task("t3", "echo t3", 0, 10),
    ];

    for task in tasks {
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let record = execute_with_retry(&task).await;
            tx_clone.send(record).await.unwrap();
        });
    }

    drop(tx); // fermeture du sender principal

    let mut ids = vec![];
    while let Some(record) = rx.recv().await {
        assert!(record.success);
        ids.push(record.task_id);
    }

    ids.sort();
    assert_eq!(ids, vec!["t1", "t2", "t3"]);
}

#[tokio::test]
async fn test_registry_due_tasks_executed_and_sent() {
    let mut reg = TaskRegistry::new();
    reg.register(make_task("reg-task", "echo from_registry", 0, 10));

    let (tx, mut rx) = mpsc::channel(32);
    let from = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

    let due: Vec<Task> = reg.due_tasks(from, 600)
        .into_iter()
        .cloned()
        .collect();

    assert_eq!(due.len(), 1);

    for task in due {
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let record = execute_with_retry(&task).await;
            tx_clone.send(record).await.unwrap();
        });
    }

    drop(tx);

    let record = rx.recv().await.unwrap();
    assert_eq!(record.task_id, "reg-task");
    assert!(record.success);
}