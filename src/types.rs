use chrono::{DateTime, Utc};
use std::fmt;

/// Représente une planification d'exécution
#[derive(Debug, Clone, PartialEq)]
pub enum Schedule {
    /// Expression cron avec 5 champs
    Cron {
        minute: CronField,
        hour: CronField,
        day_of_month: CronField,
        month: CronField,
        day_of_week: CronField,
    },
    /// Macros prédéfinies
    Daily,
    Hourly,
    Weekly,
    Monthly,
    Yearly,
    EveryMinutes(u32),
}

/// Un champ d'expression cron (peut être une valeur, liste, plage, ou étoile)
#[derive(Debug, Clone, PartialEq)]
pub enum CronField {
    Any,                           // *
    Single(u32),                   // 5
    List(Vec<u32>),               // 1,3,5
    Range(u32, u32),              // 1-5
    Step(u32, u32),               // */15 ou 1-10/2
}

/// Définit une tâche planifiée
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub command: String,
    pub schedule: Schedule,
    pub timeout_seconds: u32,
    pub max_retries: u32,
    pub enabled: bool,
}

/// Enregistrement d'une exécution 
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    pub task_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

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
            CronField::Step(_base, step) => write!(f, "*/{}", step),
        }
    }
}

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
                write!(f, "{} {} {} {} {}", 
                    minute, hour, day_of_month, month, day_of_week)
            }
        }
    }
}