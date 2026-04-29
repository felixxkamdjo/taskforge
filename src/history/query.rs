use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub task_id: String,
    pub start_time: String,
    pub end_time: String,
    pub status: String,
    pub exit_code: i32,
    pub duration_ms: i64,
    pub message: String,
}

fn parse_line(task_id: &str, line: &str) -> Option<LogEntry> {
    let parts: Vec<&str> = line.splitn(6, ',').collect();

    if parts.len() < 6 {
        return None; // ligne incomplète ou malformée
    }

    let duration_ms = parts[4].trim_end_matches("ms").parse::<i64>().unwrap_or(-1);

    let message = parts[5].trim().trim_matches('"').to_string();

    let exit_code = parts[3].parse::<i32>().unwrap_or(-1);

    Some(LogEntry {
        task_id: task_id.to_string(),
        start_time: parts[0].to_string(),
        end_time: parts[1].to_string(),
        status: parts[2].to_string(),
        exit_code,
        duration_ms,
        message,
    })
}

fn read_all_entries(task_id: &str) -> Vec<LogEntry> {
    let logs_dir = Path::new("logs");

    if !logs_dir.exists() {
        return vec![];
    }

    let mut files: Vec<_> = match fs::read_dir(logs_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|name| {
                        name.starts_with(&format!("{}_", task_id)) && name.ends_with(".log")
                    })
                    .unwrap_or(false)
            })
            .collect(),
        Err(_) => return vec![],
    };

    // Tri alphabétique = tri chronologique (grâce au format YYYY-MM)
    files.sort();

    // Lire chaque fichier et parser les lignes valides
    let mut entries = Vec::new();
    for path in files {
        match fs::read_to_string(&path) {
            Ok(content) => {
                for line in content.lines() {
                    if line.is_empty() {
                        continue;
                    }
                    if let Some(entry) = parse_line(task_id, line) {
                        entries.push(entry);
                    }
                }
            }
            Err(e) => {
                // On log l'erreur mais on continue avec les autres fichiers
                eprintln!("[history/query] Impossible de lire {:?} : {}", path, e);
            }
        }
    }

    entries
}

pub fn last_execution(task_id: &str) -> Option<LogEntry> {
    read_all_entries(task_id).into_iter().last()
}

pub fn success_rate(task_id: &str, last_n: usize) -> f64 {
    let entries = read_all_entries(task_id);

    if entries.is_empty() {
        return 0.0;
    }

    let window: Vec<_> = if last_n == 0 || last_n >= entries.len() {
        entries.iter().collect()
    } else {
        entries.iter().rev().take(last_n).collect()
    };

    let total = window.len();
    let successes = window.iter().filter(|e| e.status == "success").count();

    successes as f64 / total as f64
}

pub fn list_known_tasks() -> Vec<String> {
    let logs_dir = Path::new("logs");

    if !logs_dir.exists() {
        return vec![];
    }

    let mut task_ids = HashSet::new();

    if let Ok(entries) = fs::read_dir(logs_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".log") {
                    let without_ext = &name[..name.len() - 4]; // enlever ".log"
                    let parts: Vec<&str> = without_ext.rsplitn(3, '_').collect();
                    if parts.len() == 3 {
                        task_ids.insert(parts[2].to_string());
                    }
                }
            }
        }
    }

    let mut result: Vec<String> = task_ids.into_iter().collect();
    result.sort();
    result
}

pub fn total_executions(task_id: &str) -> usize {
    read_all_entries(task_id).len()
}

// TESTS UNITAIRES

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use tempfile::tempdir;

    /// Écrit directement une ligne de log dans le dossier courant (pour les tests)
    fn write_log_line(task_id: &str, month: &str, line: &str) {
        fs::create_dir_all("logs").unwrap();
        let path = format!("logs/{}_{}.log", task_id, month);
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        writeln!(f, "{}", line).unwrap();
    }

    /// Ligne CSV valide pour les tests
    fn success_line(task_id: &str) -> String {
        format!("2026-04-26T01:00:00Z,2026-04-26T01:00:01Z,success,0,1000ms,\"OK\"",)
    }

    fn failure_line() -> String {
        "2026-04-26T02:00:00Z,2026-04-26T02:00:03Z,failure,1,3000ms,\"Erreur connexion\""
            .to_string()
    }

    #[test]
    fn test_last_execution_trouve() {
        let dir = tempdir().unwrap();
        env::set_current_dir(&dir).unwrap();

        write_log_line("my_task", "2026-04", &success_line("my_task"));

        let entry = last_execution("my_task");
        assert!(entry.is_some(), "Devrait trouver une entrée");
        assert_eq!(entry.unwrap().status, "success");
    }

    #[test]
    fn test_last_execution_retourne_la_derniere() {
        let dir = tempdir().unwrap();
        env::set_current_dir(&dir).unwrap();

        // Deux lignes dans le même fichier
        write_log_line("task_a", "2026-04", &success_line("task_a"));
        write_log_line("task_a", "2026-04", &failure_line());

        let entry = last_execution("task_a");
        assert!(entry.is_some());
        // La dernière ligne écrite est un failure
        assert_eq!(
            entry.unwrap().status,
            "failure",
            "Devrait retourner la DERNIÈRE entrée (failure)"
        );
    }

    #[test]
    fn test_last_execution_aucun_log() {
        let dir = tempdir().unwrap();
        env::set_current_dir(&dir).unwrap();

        let entry = last_execution("tache_inexistante");
        assert!(entry.is_none(), "Devrait retourner None si aucun log");
    }

    #[test]
    fn test_success_rate_tous_succes() {
        let dir = tempdir().unwrap();
        env::set_current_dir(&dir).unwrap();

        for _ in 0..4 {
            write_log_line("task_ok", "2026-04", &success_line("task_ok"));
        }

        let rate = success_rate("task_ok", 10);
        assert!(
            (rate - 1.0).abs() < 0.001,
            "Taux attendu : 1.0, obtenu : {}",
            rate
        );
    }

    #[test]
    fn test_success_rate_mixte() {
        let dir = tempdir().unwrap();
        env::set_current_dir(&dir).unwrap();

        // 3 succès, 1 échec → 75%
        for _ in 0..3 {
            write_log_line("task_mix", "2026-04", &success_line("task_mix"));
        }
        write_log_line("task_mix", "2026-04", &failure_line());

        let rate = success_rate("task_mix", 10);
        assert!(
            (rate - 0.75).abs() < 0.001,
            "Taux attendu : 0.75, obtenu : {}",
            rate
        );
    }

    #[test]
    fn test_success_rate_fenetre_glissante() {
        let dir = tempdir().unwrap();
        env::set_current_dir(&dir).unwrap();

        // 5 succès anciens, puis 1 échec récent
        for _ in 0..5 {
            write_log_line("task_win", "2026-04", &success_line("task_win"));
        }
        write_log_line("task_win", "2026-04", &failure_line());

        // Sur les 2 dernières : 1 succès + 1 échec = 50%
        let rate = success_rate("task_win", 2);
        assert!(
            (rate - 0.5).abs() < 0.001,
            "Fenêtre sur 2 : taux attendu 0.5, obtenu {}",
            rate
        );
    }

    #[test]
    fn test_success_rate_aucun_log() {
        let dir = tempdir().unwrap();
        env::set_current_dir(&dir).unwrap();

        let rate = success_rate("inexistant", 10);
        assert_eq!(rate, 0.0, "Taux doit être 0.0 si aucun log");
    }

    #[test]
    fn test_list_known_tasks() {
        let dir = tempdir().unwrap();
        env::set_current_dir(&dir).unwrap();

        write_log_line("backup_db", "2026-04", &success_line("backup_db"));
        write_log_line("cleanup_tmp", "2026-04", &failure_line());
        write_log_line("backup_db", "2026-03", &success_line("backup_db")); // 2ème mois

        let tasks = list_known_tasks();
        assert_eq!(
            tasks.len(),
            2,
            "Attendu 2 tâches distinctes, trouvé : {:?}",
            tasks
        );
        assert!(tasks.contains(&"backup_db".to_string()));
        assert!(tasks.contains(&"cleanup_tmp".to_string()));
    }

    #[test]
    fn test_total_executions() {
        let dir = tempdir().unwrap();
        env::set_current_dir(&dir).unwrap();

        for _ in 0..7 {
            write_log_line("task_count", "2026-04", &success_line("task_count"));
        }

        assert_eq!(total_executions("task_count"), 7);
    }

    #[test]
    fn test_parse_line_valide() {
        let line = "2026-04-26T01:00:00Z,2026-04-26T01:00:01Z,success,0,1250ms,\"Backup OK\"";
        let entry = parse_line("backup_db", line);
        assert!(entry.is_some());
        let e = entry.unwrap();
        assert_eq!(e.status, "success");
        assert_eq!(e.exit_code, 0);
        assert_eq!(e.duration_ms, 1250);
        assert_eq!(e.message, "Backup OK");
    }

    #[test]
    fn test_parse_line_malformee_ignoree() {
        // Une ligne avec moins de 6 colonnes doit être ignorée
        let line = "2026-04-26,failure,1";
        let entry = parse_line("task", line);
        assert!(entry.is_none(), "Ligne malformée doit retourner None");
    }
}
