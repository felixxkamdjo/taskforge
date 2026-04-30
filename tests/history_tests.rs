use taskforge::history::{last_execution, success_rate, list_known_tasks, total_executions};
use taskforge::history::store::save_record;
use taskforge::types::ExecutionRecord;
use chrono::{Utc};
use std::env;
use std::sync::Mutex;
use tempfile::tempdir;

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn make_record(task_id: &str, success: bool, exit_code: Option<i32>) -> ExecutionRecord {
    let start = Utc::now();
    ExecutionRecord {
        task_id: task_id.to_string(),
        start_time: start,
        end_time: Some(start + chrono::Duration::milliseconds(500)),
        exit_code,
        stdout: if success { "ok".to_string() } else { String::new() },
        stderr: if !success { "erreur".to_string() } else { String::new() },
        success,
    }
}

fn make_record_at(task_id: &str, success: bool, rfc3339: &str) -> ExecutionRecord {
    let start = chrono::DateTime::parse_from_rfc3339(rfc3339)
        .unwrap()
        .with_timezone(&Utc);

    ExecutionRecord {
        task_id: task_id.to_string(),
        start_time: start,
        end_time: Some(start + chrono::Duration::seconds(1)),
        exit_code: if success { Some(0) } else { Some(1) },
        stdout: if success { "ok".to_string() } else { String::new() },
        stderr: if !success { "fail".to_string() } else { String::new() },
        success,
    }
}

// ---------------------------------------------------------------------------
// Persistance et récupération de la dernière exécution
// ---------------------------------------------------------------------------

#[test]
fn test_save_then_last_execution() {
    let _lock = TEST_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    env::set_current_dir(&dir).unwrap();

    let record = make_record("task_a", true, Some(0));
    save_record(&record).unwrap();

    let last = last_execution("task_a");
    assert!(last.is_some());
    assert_eq!(last.unwrap().status, "success");
}

#[test]
fn test_last_execution_returns_most_recent() {
    let _lock = TEST_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    env::set_current_dir(&dir).unwrap();

    save_record(&make_record_at("chrono", true,  "2026-03-10T01:00:00Z")).unwrap();
    save_record(&make_record_at("chrono", false, "2026-04-20T01:00:00Z")).unwrap();

    let last = last_execution("chrono").unwrap();
    assert_eq!(last.status, "failure",
        "Doit correspondre à l’entrée la plus récente");
}

#[test]
fn test_last_execution_unknown_task_returns_none() {
    let _lock = TEST_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    env::set_current_dir(&dir).unwrap();

    assert!(last_execution("fantome").is_none());
}

// ---------------------------------------------------------------------------
// Persistance et calcul du taux de succès
// ---------------------------------------------------------------------------

#[test]
fn test_success_rate_after_saves() {
    let _lock = TEST_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    env::set_current_dir(&dir).unwrap();

    for _ in 0..3 {
        save_record(&make_record("rate_task", true, Some(0))).unwrap();
    }
    save_record(&make_record("rate_task", false, Some(1))).unwrap();

    let rate = success_rate("rate_task", 0);
    assert!(
        (rate - 0.75).abs() < 0.001,
        "Taux attendu 0.75, obtenu {}", rate
    );
}

#[test]
fn test_success_rate_sliding_window() {
    let _lock = TEST_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    env::set_current_dir(&dir).unwrap();

    for _ in 0..5 {
        save_record(&make_record("win_task", true, Some(0))).unwrap();
    }
    for _ in 0..2 {
        save_record(&make_record("win_task", false, Some(1))).unwrap();
    }

    let rate = success_rate("win_task", 2);
    assert!(
        rate.abs() < 0.001,
        "Fenêtre récente : taux attendu 0.0, obtenu {}", rate
    );
}

#[test]
fn test_success_rate_no_history_returns_zero() {
    let _lock = TEST_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    env::set_current_dir(&dir).unwrap();

    assert_eq!(success_rate("rien", 10), 0.0);
}

// ---------------------------------------------------------------------------
// Rotation mensuelle et agrégation
// ---------------------------------------------------------------------------

#[test]
fn test_monthly_rotation_creates_separate_files() {
    let _lock = TEST_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    env::set_current_dir(&dir).unwrap();

    save_record(&make_record_at("backup", true, "2026-03-15T01:00:00Z")).unwrap();
    save_record(&make_record_at("backup", true, "2026-04-15T01:00:00Z")).unwrap();

    let files: Vec<_> = std::fs::read_dir(dir.path().join("logs"))
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(files.len(), 2,
        "Deux mois distincts doivent produire deux fichiers");
}

#[test]
fn test_list_known_tasks_after_saves() {
    let _lock = TEST_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    env::set_current_dir(&dir).unwrap();

    save_record(&make_record("backup_db",   true,  Some(0))).unwrap();
    save_record(&make_record("cleanup_tmp", false, Some(1))).unwrap();
    save_record(&make_record("backup_db",   true,  Some(0))).unwrap();

    let tasks = list_known_tasks();
    assert_eq!(tasks.len(), 2);
    assert!(tasks.contains(&"backup_db".to_string()));
    assert!(tasks.contains(&"cleanup_tmp".to_string()));
}

#[test]
fn test_total_executions_counts_all() {
    let _lock = TEST_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    env::set_current_dir(&dir).unwrap();

    for _ in 0..6 {
        save_record(&make_record("counter", true, Some(0))).unwrap();
    }

    assert_eq!(total_executions("counter"), 6);
}

#[test]
fn test_total_executions_across_months() {
    let _lock = TEST_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    env::set_current_dir(&dir).unwrap();

    for i in 0..3 {
        save_record(&make_record_at(
            "multi_month", true,
            &format!("2026-03-{:02}T01:00:00Z", i + 1)
        )).unwrap();
    }
    for i in 0..4 {
        save_record(&make_record_at(
            "multi_month", false,
            &format!("2026-04-{:02}T01:00:00Z", i + 1)
        )).unwrap();
    }

    assert_eq!(total_executions("multi_month"), 7,
        "Le total doit agréger tous les mois");
}

// ---------------------------------------------------------------------------
// Pipeline asynchrone via canal mpsc
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_history_writer_receives_and_saves() {
    let _lock = TEST_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    env::set_current_dir(&dir).unwrap();

    use tokio::sync::mpsc;
    use taskforge::history::start_history_writer;

    let (tx, rx) = mpsc::channel(32);
    start_history_writer(rx);

    let records = vec![
        make_record("writer_task", true,  Some(0)),
        make_record("writer_task", false, Some(1)),
        make_record("writer_task", true,  Some(0)),
    ];

    for r in records {
        tx.send(r).await.unwrap();
    }
    drop(tx);

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    assert_eq!(total_executions("writer_task"), 3);

    let rate = success_rate("writer_task", 0);
    assert!(
        (rate - 2.0/3.0).abs() < 0.01,
        "Taux attendu ~0.667, obtenu {}", rate
    );
}