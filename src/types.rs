use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Schedule {
    pub expression: String,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub command: String,
    pub schedule: Schedule,
}

#[derive(Debug)]
pub struct ExecutionRecord {
    pub task_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub success: bool,
    pub output: String,
}