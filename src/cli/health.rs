use crate::registry::TaskRegistry;
use crate::history::query::{last_execution, success_rate};
// use chrono::Utc;

pub fn print_health_report(registry: &TaskRegistry) {
    println!("=== RAPPORT DE SANTÉ TASKFORGE ===");
    println!("{:<15} | {:<10} | {:<10} | {:<25}", "ID Tâche", "État", "Succès (%)", "Dernière Exécution");
    println!("{:-<15}-+-{:-<10}-+-{:-<10}-+-{:-<25}", "", "", "", "");

    let tasks = registry.all_tasks();

    for task in tasks {
        let status = if task.enabled { "Actif" } else { "Désactivé" };
        
        // On calcule le taux de succès sur les 10 dernières exécutions 
        let rate = success_rate(&task.id, 10) * 100.0;
        
        // On récupère la dernière exécution 
        let last_exec = match last_execution(&task.id) {
            Some(entry) => format!("{} ({})", entry.start_time, entry.status),
            None => "Jamais exécutée".to_string(),
        };

        println!("{:<15} | {:<10} | {:<10.1} | {:<25}", task.id, status, rate, last_exec);
    }
    
    println!("\nTotal tâches actives : {} / {}", registry.active_count(), registry.len());
}

// TESTS UNITAIRES POUR LE RAPPORT DE SANTÉ

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::TaskRegistry;
    use crate::types::{Task, Schedule};

    #[test]
    fn test_print_health_report_empty_registry() {
        // On crée un registre vide
        let registry = TaskRegistry::new();
        
        // On appelle la fonction. Si le code contient un bug majeur (comme une division par zéro non gérée),
        // ce test échouera (panic). Sinon, il passera avec succès.
        print_health_report(&registry);
    }

    fn make_task(id: &str, enabled: bool) -> Task {
        Task {
            id: id.to_string(),
            name: id.to_string(),
            command: format!("echo {}", id),
            schedule: Schedule::EveryMinutes(5),
            timeout_seconds: 60,
            max_retries: 0,
            enabled,
        }
    }

    // Vérifie l'absence de panic sur registre vide
    #[test]
    fn test_health_report_empty_registry_no_panic() {
        let registry = TaskRegistry::new();
        print_health_report(&registry); // ne doit pas paniquer
    }

    // Vérifie l'absence de panic avec tâches sans historique
    #[test]
    fn test_health_report_tasks_no_history_no_panic() {
        let mut registry = TaskRegistry::new();
        registry.register(make_task("t1", true));
        registry.register(make_task("t2", false));
        // last_execution retourne None → branche "Jamais exécutée"
        print_health_report(&registry);
    }

    // Vérifie que les tâches désactivées n'impactent pas le total actif
    #[test]
    fn test_active_count_reflected_correctly() {
        let mut registry = TaskRegistry::new();
        registry.register(make_task("active", true));
        registry.register(make_task("inactive", false));
        assert_eq!(registry.active_count(), 1);
        assert_eq!(registry.len(), 2);
        // le rapport ne doit pas paniquer sur ce mix
        print_health_report(&registry);
    }
}