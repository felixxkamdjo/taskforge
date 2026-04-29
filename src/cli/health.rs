use crate::registry::TaskRegistry;
use crate::history::query::{last_execution, success_rate};
use chrono::Utc;

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

    #[test]
    fn test_print_health_report_empty_registry() {
        // On crée un registre vide
        let registry = TaskRegistry::new();
        
        // On appelle la fonction. Si le code contient un bug majeur (comme une division par zéro non gérée),
        // ce test échouera (panic). Sinon, il passera avec succès.
        print_health_report(&registry);
    }
}