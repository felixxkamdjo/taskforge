use chrono::{DateTime, Utc};
use std::fmt;

// Types de données pour les tâches et les horaires
#[derive(Debug, Clone, PartialEq)]
pub enum Schedule {
    Cron { // Expression cron à 5 champs
        minute: CronField,
        hour: CronField,
        day_of_month: CronField,
        month: CronField,
        day_of_week: CronField,
    },
    Daily,
    Hourly,
    Weekly,
    Monthly,
    Yearly,
    EveryMinutes(u32), // Intervalle en minutes
}

// Types de champs pour les expressions cron
#[derive(Debug, Clone, PartialEq)]
pub enum CronField {
    Any,             // *
    Single(u32),     // valeur unique
    List(Vec<u32>),  // liste de valeurs
    Range(u32, u32), // intervalle
    Step(u32, u32),  // base/step
}

// Types de données pour les tâches et les exécutions
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub command: String,
    pub schedule: Schedule,
    pub timeout_seconds: u32,
    pub max_retries: u32,
    pub enabled: bool, // activation de la tâche
}

// Enregistrement d'exécution pour l'historique
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    pub task_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>, // None si en cours
    pub exit_code: Option<i32>,          // None si non terminé
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

// Affichage pour debug et logs
impl fmt::Display for CronField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CronField::Any => write!(f, "*"),
            CronField::Single(v) => write!(f, "{}", v),
            CronField::List(values) => {
                let s: Vec<String> = values.iter().map(|v| v.to_string()).collect();
                write!(f, "{}", s.join(","))
            }
            CronField::Range(start, end) => write!(f, "{}-{}", start, end),
            CronField::Step(base, step) => {
                if *base == 0 {
                    write!(f, "*/{}", step) // wildcard avec pas
                } else {
                    write!(f, "{}/{}", base, step) // base explicite
                }
            }
        }
    }
}

// Affichage de l'horaire pour debug et logs
impl fmt::Display for Schedule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Schedule::Daily => write!(f, "@daily"),
            Schedule::Hourly => write!(f, "@hourly"),
            Schedule::Weekly => write!(f, "@weekly"),
            Schedule::Monthly => write!(f, "@monthly"),
            Schedule::Yearly => write!(f, "@yearly"),
            Schedule::EveryMinutes(n) => write!(f, "@every {}m", n),
            Schedule::Cron { minute, hour, day_of_month, month, day_of_week } => {
                write!(f, "{} {} {} {} {}", minute, hour, day_of_month, month, day_of_week) // format cron standard
            }
        }
    }
}