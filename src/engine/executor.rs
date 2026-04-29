use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use chrono::Utc;

use crate::types::{Task, ExecutionRecord};

/// Résultat brut d'une exécution avant retry
#[derive(Debug)]
pub struct ExecutionAttempt {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub success: bool,   // exit 0 + fin normale
    pub timed_out: bool, // arrêt forcé par timeout
}

/// Exécute une commande dans un sous-processus.
///
/// Utilise `sh -c` pour supporter pipes et redirections.
/// Capture stdout/stderr séparément.
/// Tue le processus si le timeout est dépassé.
pub async fn run_command(task: &Task) -> ExecutionAttempt {
    let timeout_duration = Duration::from_secs(task.timeout_seconds as u64);

    let child_result = Command::new("sh")
        .arg("-c")
        .arg(&task.command)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let child = match child_result {
        Ok(c) => c,
        Err(e) => {
            return ExecutionAttempt {
                stdout: String::new(),
                stderr: format!("Impossible de lancer la commande : {}", e),
                exit_code: None,
                success: false,
                timed_out: false,
            };
        }
    };

    let pid = child.id();

    // attente avec limite de temps
    match timeout(timeout_duration, child.wait_with_output()).await {
        Ok(Ok(output)) => {
            let exit_code = output.status.code();
            let success = exit_code == Some(0);

            ExecutionAttempt {
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                exit_code,
                success,
                timed_out: false,
            }
        }

        Ok(Err(e)) => {
            ExecutionAttempt {
                stdout: String::new(),
                stderr: format!("Erreur d'attente du processus : {}", e),
                exit_code: None,
                success: false,
                timed_out: false,
            }
        }

        Err(_) => {
            if let Some(pid) = pid {
                let _ = tokio::process::Command::new("kill")
                    .arg("-9")
                    .arg(pid.to_string())
                    .status()
                    .await; // arrêt forcé
            }

            ExecutionAttempt {
                stdout: String::new(),
                stderr: format!(
                    "Tâche '{}' tuée après {}s (timeout)",
                    task.id, task.timeout_seconds
                ),
                exit_code: None,
                success: false,
                timed_out: true,
            }
        }
    }
}

/// Conversion du résultat brut vers un enregistrement persistant
pub fn attempt_to_record(
    task: &Task,
    attempt: ExecutionAttempt,
    start: chrono::DateTime<Utc>,
) -> ExecutionRecord {
    ExecutionRecord {
        task_id: task.id.clone(),
        start_time: start,
        end_time: Some(Utc::now()),
        exit_code: attempt.exit_code,
        stdout: attempt.stdout,
        stderr: attempt.stderr,
        success: attempt.success,
    }
}