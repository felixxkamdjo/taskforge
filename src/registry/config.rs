use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::parser::{parse_cron_expression, parse_macro};
use crate::types::{Schedule, Task};

// Structures de configuration pour le parsing du fichier TOML
#[derive(Debug, Deserialize)]
pub struct TaskConfig {
    pub id: String,
    pub name: String,
    pub command: String,
    pub schedule: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
    #[serde(default = "default_retries")]
    pub max_retries: u32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

// Valeurs par défaut pour les champs optionnels
fn default_timeout() -> u32 {
    60 // secondes
}
fn default_retries() -> u32 {
    0
}
fn default_enabled() -> bool {
    true
}

// Configuration globale pour le parsing du fichier TOML
#[derive(Debug, Deserialize)]
pub struct TomlConfig {
    #[serde(rename = "task")]
    pub tasks: Vec<TaskConfig>, // liste des tâches
}

// erreurs possibles lors du chargement de la configuration
#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error), // erreur lecture fichier
    Parse(toml::de::Error), // erreur parsing TOML
    InvalidSchedule { task_id: String, schedule: String },
    DuplicateId(String), // id déjà présent
}

// implémentation de l'affichage des erreurs pour une meilleure lisibilité
impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "Erreur I/O : {}", e),
            ConfigError::Parse(e) => write!(f, "TOML invalide : {}", e),
            ConfigError::InvalidSchedule { task_id, schedule } => {
                write!(f, "Planification invalide pour la tâche '{}' : '{}'", task_id, schedule)
            }
            ConfigError::DuplicateId(id) => write!(f, "Identifiant dupliqué : '{}'", id),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::Parse(e)
    }
}

// fonction de parsing de la planification (cron ou macro)
pub fn parse_schedule(s: &str) -> Option<Schedule> {
    if s.starts_with('@') {
        parse_macro(s) // macro (@daily…)
    } else {
        parse_cron_expression(s).ok() // cron classique
    }
}

// conversion d'une configuration de tâche en structure interne Task
fn task_config_to_task(cfg: TaskConfig) -> Result<Task, ConfigError> {
    let schedule = parse_schedule(&cfg.schedule).ok_or_else(|| ConfigError::InvalidSchedule {
        task_id: cfg.id.clone(),
        schedule: cfg.schedule.clone(),
    })?; // validation schedule

    Ok(Task {
        id: cfg.id,
        name: cfg.name,
        command: cfg.command,
        schedule,
        timeout_seconds: cfg.timeout_seconds,
        max_retries: cfg.max_retries,
        enabled: cfg.enabled,
    })
}

// chargement et parsing du fichier de configuration TOML
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Vec<Task>, ConfigError> {
    let content = fs::read_to_string(path)?;
    let raw: TomlConfig = toml::from_str(&content)?;

    let mut seen_ids = std::collections::HashSet::new(); // détection doublons
    for cfg in &raw.tasks {
        if !seen_ids.insert(cfg.id.clone()) {
            return Err(ConfigError::DuplicateId(cfg.id.clone()));
        }
    }

    raw.tasks.into_iter().map(task_config_to_task).collect() // conversion finale
}

// tests unitaires pour la validation du parsing de la configuration
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_toml(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", content).unwrap(); // écrit contenu test
        f
    }

    // test de chargement d'une configuration valide
    #[test]
    fn test_load_valid_config() {
        let toml = r#"
[[task]]
id = "backup"
name = "Sauvegarde BDD"
command = "pg_dump mydb > /backup/db.sql"
schedule = "0 1 * * *"
timeout_seconds = 120
max_retries = 2
enabled = true
"#;
        let f = write_toml(toml);
        let tasks = load_config(f.path()).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "backup");
        assert_eq!(tasks[0].timeout_seconds, 120);
        assert_eq!(tasks[0].max_retries, 2);
        assert!(tasks[0].enabled);
    }

    // test de chargement d'une configuration avec une macro de planification
    #[test]
    fn test_load_macro_schedule() {
        let toml = r#"
[[task]]
id = "cleanup"
name = "Nettoyage"
command = "rm -rf /tmp/*"
schedule = "@daily"
"#;
        let f = write_toml(toml);
        let tasks = load_config(f.path()).unwrap();
        assert_eq!(tasks[0].schedule, crate::types::Schedule::Daily);
    }

    // test de chargement d'une configuration avec des valeurs par défaut
    #[test]
    fn test_default_values() {
        let toml = r#"
[[task]]
id = "ping"
name = "Ping"
command = "ping -c1 localhost"
schedule = "@hourly"
"#;
        let f = write_toml(toml);
        let tasks = load_config(f.path()).unwrap();
        assert_eq!(tasks[0].timeout_seconds, 60); // valeurs par défaut
        assert_eq!(tasks[0].max_retries, 0);
        assert!(tasks[0].enabled);
    }

    // test de chargement d'une configuration avec une planification invalide
    #[test]
    fn test_invalid_schedule_returns_error() {
        let toml = r#"
[[task]]
id = "bad"
name = "Mauvaise tâche"
command = "echo bad"
schedule = "pas_valide"
"#;
        let f = write_toml(toml);
        let result = load_config(f.path());
        assert!(result.is_err()); // doit échouer
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidSchedule { .. }));
    }

    // test de chargement d'une configuration avec des identifiants dupliqués
    #[test]
    fn test_duplicate_id_returns_error() {
        let toml = r#"
[[task]]
id = "dup"
name = "Tâche 1"
command = "echo 1"
schedule = "@daily"

[[task]]
id = "dup"
name = "Tâche 2"
command = "echo 2"
schedule = "@hourly"
"#;
        let f = write_toml(toml);
        let result = load_config(f.path());
        assert!(matches!(result.unwrap_err(), ConfigError::DuplicateId(_))); // doublon détecté
    }

    // test de chargement d'une configuration avec plusieurs tâches et vérification de l'état d'activation
    #[test]
    fn test_multiple_tasks() {
        let toml = r#"
[[task]]
id = "t1"
name = "Tâche 1"
command = "echo 1"
schedule = "*/15 * * * *"

[[task]]
id = "t2"
name = "Tâche 2"
command = "echo 2"
schedule = "@weekly"
enabled = false
"#;
        let f = write_toml(toml);
        let tasks = load_config(f.path()).unwrap();
        assert_eq!(tasks.len(), 2);
        assert!(!tasks[1].enabled); // état respecté
    }

    // test de chargement d'une configuration avec une macro @every et vérification de l'intervalle
    #[test]
    fn test_every_macro_schedule() {
        let toml = r#"
[[task]]
id = "uptime"
name = "Vérification uptime"
command = "systemctl status nginx"
schedule = "@every 5m"
"#;
        let f = write_toml(toml);
        let tasks = load_config(f.path()).unwrap();
        assert_eq!(tasks[0].schedule, crate::types::Schedule::EveryMinutes(5));
    }

    // test de chargement d'une configuration avec une macro @every et vérification de l'intervalle en heures
    #[test]
    fn test_file_not_found() {
        let result = load_config("/fichier/qui/nexiste/pas.toml");
        assert!(matches!(result.unwrap_err(), ConfigError::Io(_))); // erreur I/O
    }

    // test de chargement d'une configuration avec un TOML mal formé
    #[test]
    fn test_malformed_toml() {
        let f = write_toml("ceci n'est pas du TOML valide [[[");
        let result = load_config(f.path());
        assert!(matches!(result.unwrap_err(), ConfigError::Parse(_))); // erreur parsing
    }
}