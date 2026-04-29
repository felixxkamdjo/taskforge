use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::types::ExecutionRecord;

fn log_path(record: &ExecutionRecord) -> PathBuf {
    // On utilise start_time pour déterminer le mois de l'exécution
    let month = record.start_time.format("%Y-%m").to_string();
    PathBuf::from("logs").join(format!("{}_{}.log", record.task_id, month))
}

fn to_csv_line(record: &ExecutionRecord) -> String {
    // Formatage de end_time (peut être None si tâche interrompue)
    let end_time_str = match record.end_time {
        Some(t) => t.to_rfc3339(),
        None => "en_cours".to_string(),
    };

    let status = if record.success { "success" } else { "failure" };

    let exit_code = record.exit_code.unwrap_or(-1);

    // Durée en millisecondes (calculée si end_time disponible)
    let duration_ms = match record.end_time {
        Some(end) => (end - record.start_time).num_milliseconds(),
        None => -1, // durée inconnue
    };

    let message = if !record.success && !record.stderr.is_empty() {
        record.stderr.replace('"', "'").replace('\n', " ")
    } else {
        record.stdout.replace('"', "'").replace('\n', " ")
    };

    format!(
        "{},{},{},{},{}ms,\"{}\"\n",
        record.start_time.to_rfc3339(),
        end_time_str,
        status,
        exit_code,
        duration_ms,
        message
    )
}

pub fn save_record(record: &ExecutionRecord) -> std::io::Result<()> {
    // 1. S'assurer que le dossier logs/ existe
    fs::create_dir_all("logs")?;

    // 2. Déterminer le fichier cible (rotation mensuelle automatique)
    let path = log_path(record);

    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;

    let line = to_csv_line(record);
    file.write_all(line.as_bytes())?;

    Ok(())
}

pub fn start_history_writer(mut rx: mpsc::Receiver<ExecutionRecord>) {
    tokio::spawn(async move {
        while let Some(record) = rx.recv().await {
            let task_id = record.task_id.clone();
            let status = if record.success {
                "✓ succès"
            } else {
                "✗ échec"
            };

            match save_record(&record) {
                Ok(_) => {
                    println!("[history] Enregistré : {} — {}", task_id, status);
                }
                Err(e) => {
                    eprintln!("[history] ERREUR écriture pour '{}' : {}", task_id, e);
                }
            }
        }

        println!("[history] Canal fermé — writer arrêté proprement.");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::env;
    use tempfile::tempdir;

    /// Crée un ExecutionRecord de test avec les paramètres donnés
    fn make_record(task_id: &str, success: bool, exit_code: Option<i32>) -> ExecutionRecord {
        let start = Utc::now();
        let end = start + chrono::Duration::milliseconds(800);
        ExecutionRecord {
            task_id: task_id.to_string(),
            start_time: start,
            end_time: Some(end),
            exit_code,
            stdout: if success {
                "tout ok".to_string()
            } else {
                String::new()
            },
            stderr: if !success {
                "erreur critique".to_string()
            } else {
                String::new()
            },
            success,
        }
    }

    #[test]
    fn test_log_path_format_mensuel() {
        let record = make_record("backup_db", true, Some(0));
        let path = log_path(&record);
        let name = path.file_name().unwrap().to_str().unwrap();

        assert!(name.starts_with("backup_db_"), "Nom incorrect : {}", name);
        assert!(name.ends_with(".log"), "Extension incorrecte : {}", name);
        assert!(name.contains("202"), "Année manquante dans : {}", name);
    }

    #[test]
    fn test_csv_line_succes() {
        let record = make_record("ping", true, Some(0));
        let line = to_csv_line(&record);

        assert!(line.contains("success"), "Statut manquant : {}", line);
        assert!(line.contains(",0,"), "Exit code 0 manquant : {}", line);
        assert!(line.contains("ms,"), "Durée manquante : {}", line);
        assert!(line.ends_with('\n'), "Newline finale manquante");
    }

    #[test]
    fn test_csv_line_echec_avec_stderr() {
        let record = make_record("backup", false, Some(1));
        let line = to_csv_line(&record);

        assert!(
            line.contains("failure"),
            "Statut failure manquant : {}",
            line
        );
        assert!(
            line.contains("erreur critique"),
            "Message stderr manquant : {}",
            line
        );
    }

    #[test]
    fn test_csv_line_timeout_end_time_none() {
        let mut record = make_record("slow_task", false, None);
        record.end_time = None; // simuler un timeout

        let line = to_csv_line(&record);
        assert!(
            line.contains("en_cours"),
            "Marqueur en_cours manquant : {}",
            line
        );
        assert!(line.contains(",-1,"), "Exit code -1 manquant : {}", line);
    }

    #[test]
    fn test_save_record_cree_le_fichier() {
        // Vérifie que save_record crée bien le fichier de log
        let dir = tempdir().unwrap();
        env::set_current_dir(&dir).unwrap();

        let record = make_record("test_task", true, Some(0));
        save_record(&record).expect("save_record ne doit pas échouer");

        // Le dossier logs/ doit avoir été créé
        assert!(dir.path().join("logs").exists(), "Dossier logs/ non créé");

        // Au moins un fichier .log doit exister
        let logs: Vec<_> = fs::read_dir(dir.path().join("logs"))
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert!(!logs.is_empty(), "Aucun fichier de log créé");
    }

    #[test]
    fn test_save_record_append_multiple() {
        let dir = tempdir().unwrap();
        env::set_current_dir(&dir).unwrap();

        for i in 0..5 {
            let mut record = make_record("task_multi", i % 2 == 0, Some(i as i32 % 2));

            record.start_time = record.start_time + chrono::Duration::seconds(i as i64);
            save_record(&record).expect("Échec save_record");
        }

        let logs_dir = dir.path().join("logs");
        let files: Vec<_> = fs::read_dir(&logs_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert!(!files.is_empty());

        let content = fs::read_to_string(files[0].path()).unwrap();
        let lines: Vec<_> = content.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 5, "Attendu 5 lignes, trouvé : {}", lines.len());
    }

    #[test]
    fn test_rotation_mensuelle_fichiers_distincts() {
        // Vérifie que deux mois différents → deux fichiers distincts
        let dir = tempdir().unwrap();
        env::set_current_dir(&dir).unwrap();

        // Mois 1 : avril 2026
        let mut r1 = make_record("backup", true, Some(0));
        r1.start_time = chrono::DateTime::parse_from_rfc3339("2026-04-15T01:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        r1.end_time = Some(r1.start_time + chrono::Duration::seconds(1));

        // Mois 2 : mars 2026
        let mut r2 = make_record("backup", false, Some(1));
        r2.start_time = chrono::DateTime::parse_from_rfc3339("2026-03-15T01:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        r2.end_time = Some(r2.start_time + chrono::Duration::seconds(1));

        save_record(&r1).unwrap();
        save_record(&r2).unwrap();

        let files: Vec<_> = fs::read_dir(dir.path().join("logs"))
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(
            files.len(),
            2,
            "Attendu 2 fichiers (rotation mensuelle), trouvé : {}",
            files.len()
        );
    }
}
