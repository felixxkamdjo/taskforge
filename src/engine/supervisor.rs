use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use chrono::Utc;

use crate::registry::TaskRegistry;
use crate::types::{Task, ExecutionRecord};
use crate::engine::executor::{run_command, attempt_to_record};
use crate::engine::retry::RetryPolicy;

const TICK_INTERVAL_SECS: u64 = 60; // fréquence du superviseur
const DUE_WINDOW_SECS: i64 = 65;    // marge de tolérance autour du tick

/// Boucle principale du superviseur.
///
/// Parcourt périodiquement les tâches dues et les exécute en parallèle.
/// Le registre est partagé pour permettre les modifications runtime.
pub async fn run_supervisor(
    registry: Arc<Mutex<TaskRegistry>>,
    tx: mpsc::Sender<ExecutionRecord>,
) {
    loop {
        let now = Utc::now();

        // extraction des tâches dues sans bloquer l'exécution
        let due_tasks: Vec<Task> = {
            let reg = registry.lock().unwrap();
            reg.due_tasks(now, DUE_WINDOW_SECS)
                .into_iter()
                .cloned()
                .collect()
        };

        for task in due_tasks {
            let tx_clone = tx.clone();

            // exécution concurrente par tâche
            tokio::spawn(async move {
                let record = execute_with_retry(&task).await;

                // envoi vers le canal (ignoré si receiver fermé)
                let _ = tx_clone.send(record).await;
            });
        }

        sleep(Duration::from_secs(TICK_INTERVAL_SECS)).await;
    }
}

/// Exécute une tâche avec retry exponentiel.
///
/// Arrêt immédiat en cas de succès ou après épuisement des retries.
/// Retourne le dernier état d'exécution.
pub async fn execute_with_retry(task: &Task) -> ExecutionRecord {
    let policy = RetryPolicy::new(task.max_retries);
    let mut attempts_done: u32 = 0;

    loop {
        let start = Utc::now();
        let attempt = run_command(task).await;
        let success = attempt.success;

        let record = attempt_to_record(task, attempt, start);

        if success {
            return record; // arrêt sur succès
        }

        attempts_done += 1;

        if !policy.should_retry(attempts_done) {
            return record; // retries épuisés
        }

        let delay = policy.delay_for(attempts_done);

        eprintln!(
            "[supervisor] '{}' échec (tentative {}). retry dans {}s",
            task.id,
            attempts_done,
            delay.as_secs()
        );

        sleep(delay).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Schedule, CronField};

    fn make_task(id: &str, command: &str, max_retries: u32, timeout_seconds: u32) -> Task {
        Task {
            id: id.to_string(),
            name: id.to_string(),
            command: command.to_string(),
            schedule: Schedule::Cron {
                minute: CronField::Any,
                hour: CronField::Any,
                day_of_month: CronField::Any,
                month: CronField::Any,
                day_of_week: CronField::Any,
            },
            timeout_seconds,
            max_retries,
            enabled: true,
        }
    }

    #[tokio::test]
    async fn test_execute_success_no_retry() {
        let task = make_task("ok", "echo ok", 3, 10);
        let record = execute_with_retry(&task).await;
        assert!(record.success);
        assert_eq!(record.task_id, "ok");
    }

    #[tokio::test]
    async fn test_execute_failure_exhausts_retries() {
        let task = make_task("fail", "exit 1", 0, 5);
        let record = execute_with_retry(&task).await;
        assert!(!record.success);
        assert_eq!(record.exit_code, Some(1));
    }

    #[tokio::test]
    async fn test_execute_timeout_then_no_retry() {
        let task = make_task("slow", "sleep 10", 0, 1);
        let record = execute_with_retry(&task).await;
        assert!(!record.success);
        assert!(record.stderr.contains("timeout"));
    }

    #[tokio::test]
    async fn test_supervisor_sends_record_via_channel() {
        let registry = Arc::new(Mutex::new(TaskRegistry::new()));
        let (tx, mut rx) = mpsc::channel::<ExecutionRecord>(32);

        {
            let mut reg = registry.lock().unwrap();
            reg.register(Task {
                id: "ping".to_string(),
                name: "ping".to_string(),
                command: "echo pong".to_string(),
                schedule: Schedule::EveryMinutes(1),
                timeout_seconds: 5,
                max_retries: 0,
                enabled: true,
            });
        }

        let task = {
            let reg = registry.lock().unwrap();
            reg.get("ping").unwrap().clone()
        };

        let record = execute_with_retry(&task).await;
        tx.send(record).await.unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.task_id, "ping");
        assert!(received.success);
    }

    #[tokio::test]
    async fn test_record_timestamps_coherent() {
        let task = make_task("ts", "echo ts", 0, 10);
        let before = Utc::now();
        let record = execute_with_retry(&task).await;
        let after = Utc::now();

        assert!(record.start_time >= before);
        assert!(record.end_time.unwrap() <= after);
        assert!(record.end_time.unwrap() >= record.start_time);
    }
}