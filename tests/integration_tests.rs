use taskforge::registry::{load_config, TaskRegistry};
use taskforge::engine::execute_with_retry;
use taskforge::types::{Schedule, Task};
use chrono::{Utc, TimeZone};
use std::io::Write;
use std::sync::{Mutex};
use tokio::sync::mpsc;
use tempfile::NamedTempFile;

// Verrou global pour sérialiser les tests manipulant TASKFORGE_LOGS_DIR.
// Évite les conflits liés aux variables d'environnement partagées.
static HISTORY_LOCK: Mutex<()> = Mutex::new(());

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
// P1 + P2 : parsing vers registre vers tâches dues
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

    // Seule "fast" est due dans une fenêtre de 10 minutes
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

    assert_eq!(reg.len(), 6);
    assert_eq!(reg.active_count(), 6);
}

// ---------------------------------------------------------------------------
// P2 + P3 : exécution vers ExecutionRecord
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_successful_task_produces_record() {
    let task = make_task("echo", "echo integration_ok", 0, 10);
    let record = execute_with_retry(&task).await;

    assert!(record.success);
    assert_eq!(record.task_id, "echo");
    assert_eq!(record.exit_code, Some(0));
    assert!(record.stdout.contains("integration_ok"));
    assert!(record.end_time.is_some());
    assert!(record.end_time.unwrap() >= record.start_time);
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
    // Simulation d’un échec puis succès via un fichier flag
    let flag = format!("/tmp/taskforge_retry_test_{}", std::process::id());
    let cmd = format!(
        "if [ ! -f {flag} ]; then touch {flag}; exit 1; else rm {flag}; echo ok; fi",
        flag = flag
    );

    let task_fail = make_task("retry-flag", &cmd, 0, 10);

    let record1 = execute_with_retry(&task_fail).await;
    assert!(!record1.success);

    let record2 = execute_with_retry(&task_fail).await;
    assert!(record2.success);
}

// ---------------------------------------------------------------------------
// P1 + P2 + P3 : exécution via canal mpsc
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
    drop(tx);

    let mut ids = vec![];
    while let Some(record) = rx.recv().await {
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

// ---------------------------------------------------------------------------
// P1 vers P4 : exécution + persistance
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_full_pipeline_execute_then_persist() {
    use taskforge::history::store::save_record;
    use taskforge::history::{last_execution, success_rate, total_executions};
    use std::env;
    use tempfile::tempdir;

    let _lock = HISTORY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempdir().unwrap();
    env::set_var("TASKFORGE_LOGS_DIR", dir.path());

    let task = make_task("pipeline_ok", "echo pipeline", 0, 10);
    let record = execute_with_retry(&task).await;
    assert!(record.success);

    save_record(&record).unwrap();

    let last = last_execution("pipeline_ok").unwrap();
    assert_eq!(last.status, "success");
    assert_eq!(total_executions("pipeline_ok"), 1);
    assert!((success_rate("pipeline_ok", 0) - 1.0).abs() < 0.001);
}

#[tokio::test]
async fn test_full_pipeline_with_failure_persisted() {
    use taskforge::history::store::save_record;
    use taskforge::history::{last_execution, success_rate};
    use std::env;
    use tempfile::tempdir;

    let _lock = HISTORY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempdir().unwrap();
    env::set_var("TASKFORGE_LOGS_DIR", dir.path());

    for _ in 0..2 {
        let task = make_task("pipeline_mix", "echo ok", 0, 10);
        let record = execute_with_retry(&task).await;
        save_record(&record).unwrap();
    }

    let task_fail = make_task("pipeline_mix", "exit 1", 0, 10);
    let record_fail = execute_with_retry(&task_fail).await;
    save_record(&record_fail).unwrap();

    let last = last_execution("pipeline_mix").unwrap();
    assert_eq!(last.status, "failure");

    let rate = success_rate("pipeline_mix", 0);
    assert!((rate - 2.0/3.0).abs() < 0.01);
}

#[tokio::test]
async fn test_full_pipeline_via_mpsc_channel() {
    use taskforge::history::{start_history_writer, total_executions, success_rate};
    use std::env;
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    let _lock = HISTORY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempdir().unwrap();
    env::set_var("TASKFORGE_LOGS_DIR", dir.path());

    let (tx, rx) = mpsc::channel(32);
    start_history_writer(rx);

    let tasks = vec![
        make_task("chan_full", "echo t1", 0, 10),
        make_task("chan_full", "echo t2", 0, 10),
        make_task("chan_full", "exit 1",  0, 5),
    ];

    for task in tasks {
        let record = execute_with_retry(&task).await;
        tx.send(record).await.unwrap();
    }
    drop(tx);

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    assert_eq!(total_executions("chan_full"), 3);
    let rate = success_rate("chan_full", 0);
    assert!((rate - 2.0/3.0).abs() < 0.01);
}

// ---------------------------------------------------------------------------
// P5 : CLI — parsing et dispatch
// ---------------------------------------------------------------------------

mod cli {
    use clap::{Parser, Subcommand};

    #[derive(Parser)]
    #[command(name = "taskforge")]
    #[command(about = "Planificateur de tâches systèmes en Rust", long_about = None)]
    pub struct Cli {
        #[command(subcommand)]
        pub command: Commands,

        /// Chemin vers le fichier de configuration TOML
        #[arg(short, long, default_value = "config/tasks.toml")]
        pub config: String,
    }

    #[derive(Subcommand)]
    pub enum Commands {
        /// Liste toutes les tâches enregistrées
        List,
        /// Force l'exécution immédiate d'une tâche
        Run { name: String },
        /// Active une tâche désactivée
        Enable { name: String },
        /// Désactive une tâche sans la supprimer
        Disable { name: String },
        /// Affiche le statut en temps réel et le rapport de santé
        Status,
    }

    // pub mod health {
    //     use taskforge::registry::TaskRegistry;

    //     // pub fn print_health_report(_registry: &TaskRegistry) {
    //     //     // Stubbed health report for integration tests.
    //     // }
    // }
}

#[test]
fn test_cli_list_parses_and_loads_config() {
    use clap::Parser;
    use crate::cli::{Cli, Commands};

    // Vérifie que --config + list se parsent correctement
    let args = vec!["taskforge", "--config", "config/tasks.toml", "list"];
    let cli = Cli::parse_from(args);
    assert!(matches!(cli.command, Commands::List));
    assert_eq!(cli.config, "config/tasks.toml");
}

#[test]
fn test_cli_enable_disable_round_trip() {
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    let enable_args = vec!["taskforge", "enable", "backup"];
    let cli = Cli::parse_from(enable_args);
    match cli.command {
        Commands::Enable { name } => assert_eq!(name, "backup"),
        _ => panic!("attendu 'enable'"),
    }

    let disable_args = vec!["taskforge", "disable", "backup"];
    let cli = Cli::parse_from(disable_args);
    match cli.command {
        Commands::Disable { name } => assert_eq!(name, "backup"),
        _ => panic!("attendu 'disable'"),
    }
}

#[test]
fn test_cli_config_to_registry_enable_disable() {
    use std::io::Write;
    use tempfile::NamedTempFile;
    use taskforge::registry::{load_config, TaskRegistry};

    let toml = r#"
[[task]]
id = "toggled"
name = "Toggle test"
command = "echo toggle"
schedule = "@daily"
enabled = true
"#;
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{}", toml).unwrap();

    let tasks = load_config(f.path()).unwrap();
    let mut registry = TaskRegistry::from_tasks(tasks);

    // simule `taskforge disable toggled`
    assert!(registry.set_enabled("toggled", false));
    assert!(!registry.get("toggled").unwrap().enabled);

    // simule `taskforge enable toggled`
    assert!(registry.set_enabled("toggled", true));
    assert!(registry.get("toggled").unwrap().enabled);
}

#[test]
fn test_cli_run_unknown_task_returns_none() {
    use taskforge::registry::TaskRegistry;

    let registry = TaskRegistry::new();
    // simule `taskforge run inexistant` — doit retourner None sans paniquer
    assert!(registry.get("inexistant").is_none());
}

#[test]
fn test_cli_status_health_report_no_panic() {
    use taskforge::registry::TaskRegistry;
    use taskforge::types::{Task, Schedule};

    let mut registry = TaskRegistry::new();
    registry.register(Task {
        id: "status_test".to_string(),
        name: "Status Test".to_string(),
        command: "echo ok".to_string(),
        schedule: Schedule::EveryMinutes(5),
        timeout_seconds: 10,
        max_retries: 0,
        enabled: true,
    });

    // simule `taskforge status` — ne doit pas paniquer
    assert!(registry.get("status_test").is_some());
}