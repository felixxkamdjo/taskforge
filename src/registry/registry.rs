use std::collections::HashMap;
use chrono::{DateTime, Utc};

use crate::parser::next_occurrence;
// use crate::types::{Schedule};
use crate::types::{Task};

// registre de tâches en mémoire, avec des méthodes pour gérer les tâches et calculer les tâches à exécuter
#[derive(Debug)]
pub struct TaskRegistry {
    tasks: HashMap<String, Task>, // id → Task
}

// implémentation des méthodes de gestion des tâches dans le registre
impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    pub fn from_tasks(tasks: Vec<Task>) -> Self {
        let mut registry = Self::new();
        for task in tasks {
            registry.register(task);
        }
        registry
    }

    pub fn register(&mut self, task: Task) -> Option<Task> {
        self.tasks.insert(task.id.clone(), task) // remplace si existe
    }

    pub fn unregister(&mut self, id: &str) -> Option<Task> {
        self.tasks.remove(id) // suppression par id
    }

    pub fn get(&self, id: &str) -> Option<&Task> {
        self.tasks.get(id)
    }

    pub fn all_tasks(&self) -> Vec<&Task> {
        self.tasks.values().collect() // vue non filtrée
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(task) = self.tasks.get_mut(id) {
            task.enabled = enabled;
            true
        } else {
            false
        }
    }

    pub fn due_tasks(&self, from: DateTime<Utc>, window_secs: i64) -> Vec<&Task> {
        let deadline = from + chrono::Duration::seconds(window_secs); // borne de la fenêtre

        self.tasks
            .values()
            .filter(|task| {
                if !task.enabled {
                    return false; // ignore tâches désactivées
                }
                match next_occurrence(&task.schedule, from) {
                    Some(next) => next <= deadline,
                    None => false,
                }
            })
            .collect()
    }

    pub fn upcoming(&self, from: DateTime<Utc>) -> Vec<(&Task, DateTime<Utc>)> {
        let mut result: Vec<(&Task, DateTime<Utc>)> = self
            .tasks
            .values()
            .filter(|t| t.enabled)
            .filter_map(|task| {
                next_occurrence(&task.schedule, from).map(|next| (task, next))
            })
            .collect();

        result.sort_by_key(|(_, next)| *next); // ordre chronologique
        result
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn active_count(&self) -> usize {
        self.tasks.values().filter(|t| t.enabled).count()
    }
}

// fonction de conversion d'une configuration brute en tâche valide
impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// implémentation de tests unitaires pour vérifier le bon fonctionnement du registre de tâches
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CronField, Schedule};
    use chrono::TimeZone;

    // fonction utilitaire pour créer des tâches de test
    fn make_task(id: &str, schedule: Schedule, enabled: bool) -> Task {
        Task {
            id: id.to_string(),
            name: format!("Tâche {}", id),
            command: format!("echo {}", id),
            schedule,
            timeout_seconds: 60,
            max_retries: 0,
            enabled,
        }
    }

    // test de l'enregistrement et de la récupération d'une tâche
    #[test]
    fn test_register_and_get() {
        let mut reg = TaskRegistry::new();
        let task = make_task("t1", Schedule::Daily, true);
        reg.register(task);
        assert!(reg.get("t1").is_some());
        assert_eq!(reg.get("t1").unwrap().id, "t1");
    }

    // test de l'enregistrement d'une tâche avec un ID existant (remplacement)
    #[test]
    fn test_register_replaces_existing() {
        let mut reg = TaskRegistry::new();
        reg.register(make_task("t1", Schedule::Daily, true));
        let old = reg.register(make_task("t1", Schedule::Hourly, false));
        assert!(old.is_some());
        assert!(!reg.get("t1").unwrap().enabled); // écrasement confirmé
    }

    // test de la suppression d'une tâche
    #[test]
    fn test_unregister() {
        let mut reg = TaskRegistry::new();
        reg.register(make_task("t1", Schedule::Daily, true));
        let removed = reg.unregister("t1");
        assert!(removed.is_some());
        assert!(reg.get("t1").is_none());
    }

    // test de la suppression d'une tâche inexistante (doit retourner None)
    #[test]
    fn test_unregister_unknown() {
        let mut reg = TaskRegistry::new();
        assert!(reg.unregister("inexistant").is_none());
    }

    // test de la récupération de toutes les tâches
    #[test]
    fn test_set_enabled() {
        let mut reg = TaskRegistry::new();
        reg.register(make_task("t1", Schedule::Daily, true));
        assert!(reg.set_enabled("t1", false));
        assert!(!reg.get("t1").unwrap().enabled);
    }

    // test de la tentative de modification de l'état d'une tâche inconnue (doit retourner false)
    #[test]
    fn test_set_enabled_unknown_task() {
        let mut reg = TaskRegistry::new();
        assert!(!reg.set_enabled("fantome", false));
    }

    // test de la récupération de toutes les tâches
    #[test]
    fn test_active_count() {
        let mut reg = TaskRegistry::new();
        reg.register(make_task("t1", Schedule::Daily, true));
        reg.register(make_task("t2", Schedule::Hourly, false));
        reg.register(make_task("t3", Schedule::Weekly, true));
        assert_eq!(reg.active_count(), 2);
        assert_eq!(reg.len(), 3);
    }

    // test de la récupération des tâches dues dans une fenêtre de temps
    #[test]
    fn test_due_tasks_finds_due() {
        let mut reg = TaskRegistry::new();
        reg.register(make_task("every5m", Schedule::EveryMinutes(5), true));
        let from = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let due = reg.due_tasks(from, 600);
        assert!(!due.is_empty());
    }

    // test de la récupération des tâches dues en excluant les tâches désactivées
    #[test]
    fn test_due_tasks_excludes_disabled() {
        let mut reg = TaskRegistry::new();
        reg.register(make_task("every5m", Schedule::EveryMinutes(5), false));
        let from = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let due = reg.due_tasks(from, 600);
        assert!(due.is_empty());
    }

    // test de la récupération des tâches dues en respectant la fenêtre de temps
    #[test]
    fn test_due_tasks_narrow_window() {
        let mut reg = TaskRegistry::new();
        reg.register(make_task("daily", Schedule::Daily, true));
        let from = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let due = reg.due_tasks(from, 60);
        assert!(due.is_empty()); // hors fenêtre
    }

    // test de la récupération des tâches à venir triées par ordre d'occurrence
    #[test]
    fn test_upcoming_sorted() {
        let mut reg = TaskRegistry::new();
        reg.register(make_task("daily", Schedule::Daily, true));
        reg.register(make_task("every5m", Schedule::EveryMinutes(5), true));
        let from = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let upcoming = reg.upcoming(from);
        assert_eq!(upcoming.len(), 2);
        assert_eq!(upcoming[0].0.id, "every5m"); // plus proche d'abord
        assert_eq!(upcoming[1].0.id, "daily");
    }

    // test de chargement d'une configuration avec une macro de planification
    #[test]
    fn test_from_tasks() {
        let tasks = vec![
            make_task("a", Schedule::Daily, true),
            make_task("b", Schedule::Hourly, true),
        ];
        let reg = TaskRegistry::from_tasks(tasks);
        assert_eq!(reg.len(), 2);
    }

    // test de la récupération des tâches dues pour une tâche cron avec un champ wildcard
    #[test]
    fn test_due_cron_task() {
        let mut reg = TaskRegistry::new();
        let task = make_task(
            "every_min",
            Schedule::Cron {
                minute: CronField::Any,
                hour: CronField::Any,
                day_of_month: CronField::Any,
                month: CronField::Any,
                day_of_week: CronField::Any,
            },
            true,
        );
        reg.register(task);
        let from = Utc.with_ymd_and_hms(2024, 6, 15, 10, 0, 0).unwrap();
        let due = reg.due_tasks(from, 120);
        assert!(!due.is_empty());
    }
}