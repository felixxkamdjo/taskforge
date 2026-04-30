use clap::{Parser, Subcommand};
use crate::registry::{TaskRegistry, load_config};
use crate::cli::health::print_health_report;

#[derive(Parser)]
#[command(name = "taskforge")]
#[command(about = "Planificateur de tâches systèmes en Rust", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Chemin vers le fichier de configuration TOML
    #[arg(short, long, default_value = "config/tasks.toml")]
    pub config: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Liste toutes les tâches enregistrées
    List,
    /// Force l'exécution immédiate d'une tâche
    Run { name: String },
    /// Active une tâche désactivée
    Enable { name: String },
    /// Désactive une tâche sans la supprimer
    Disable { name: String },
    /// Affiche le statut en temps réel et le rapport de santé
    Status,
}

pub fn execute_cli() {
    let cli = Cli::parse();

    //  On charge les données grâce au module de P2
    let tasks = match load_config(&cli.config) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Erreur lors du chargement de la configuration : {}", e);
            return;
        }
    };
    
    //  On initialise le registre de P2 avec ces tâches
    let mut registry = TaskRegistry::from_tasks(tasks);

    // On exécute la bonne commande
    match &cli.command {
        Commands::List => {
            println!("{:<15} | {:<20} | {:<20} | {:<10}", "ID", "Nom", "Planification", "État");
            println!("{:-<15}-+-{:-<20}-+-{:-<20}-+-{:-<10}", "", "", "", "");
            
            // On utilise la méthode all_tasks() de P2
            for task in registry.all_tasks() {
                let status = if task.enabled { "Actif" } else { "Inactif" };
                // L'affichage de la planification (schedule) fonctionne grâce à l'implémentation "Display" de P1
                println!("{:<15} | {:<20} | {:<20} | {:<10}", task.id, task.name, task.schedule.to_string(), status);
            }
        }
        Commands::Status => {
            // On appelle ton fichier health.rs
            print_health_report(&registry);
        }
        Commands::Enable { name } => {
            // On utilise la méthode set_enabled() de P2
            if registry.set_enabled(name, true) {
                println!("Succès : La tâche '{}' est maintenant activée.", name);
                // Note pour l'intégration J6 : Il faudra notifier le moteur (P3)
            } else {
                eprintln!("Erreur : Tâche '{}' introuvable.", name);
            }
        }
        Commands::Disable { name } => {
            if registry.set_enabled(name, false) {
                println!("Succès : La tâche '{}' est maintenant désactivée.", name);
                // Note pour l'intégration J6 : Il faudra notifier le moteur (P3)
            } else {
                eprintln!("Erreur : Tâche '{}' introuvable.", name);
            }
        }
        Commands::Run { name } => {
            if let Some(task) = registry.get(name) {
                println!("Exécution forcée de la tâche '{}' (Commande: {})...", task.id, task.command);
                // Note pour l'intégration J6 : L'exécution réelle se fera via un signal envoyé à P3
                println!("Signal envoyé au moteur d'exécution (Simulation).");
            } else {
                eprintln!("Erreur : Tâche '{}' introuvable.", name);
            }
        }
    }
}

// TESTS UNITAIRES POUR LA CLI

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    // 1. Ce test vérifie que la structure globale de ta CLI n'a pas d'erreurs de configuration interne
    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }

    // 2. Test du parsing de la commande "list"
    #[test]
    fn test_parse_list() {
        let args = vec!["taskforge", "list"];
        let cli = Cli::parse_from(args);
        assert!(matches!(cli.command, Commands::List));
    }

    // 3. Test du parsing de la commande "run" avec un argument
    #[test]
    fn test_parse_run() {
        let args = vec!["taskforge", "run", "backup_db"];
        let cli = Cli::parse_from(args);
        
        match cli.command {
            Commands::Run { name } => assert_eq!(name, "backup_db"),
            _ => panic!("La commande parsée n'est pas 'run'"),
        }
    }

    // 4. Test du parsing de la commande "enable"
    #[test]
    fn test_parse_enable() {
        let args = vec!["taskforge", "enable", "cleanup_tmp"];
        let cli = Cli::parse_from(args);
        
        match cli.command {
            Commands::Enable { name } => assert_eq!(name, "cleanup_tmp"),
            _ => panic!("La commande parsée n'est pas 'enable'"),
        }
    }

    #[test]
    fn test_parse_disable() {
        let args = vec!["taskforge", "disable", "nightly_backup"];
        let cli = Cli::parse_from(args);
        match cli.command {
            Commands::Disable { name } => assert_eq!(name, "nightly_backup"),
            _ => panic!("attendu 'disable'"),
        }
    }

    #[test]
    fn test_parse_status() {
        let args = vec!["taskforge", "status"];
        let cli = Cli::parse_from(args);
        assert!(matches!(cli.command, Commands::Status));
    }

    #[test]
    fn test_parse_custom_config_path() {
        let args = vec!["taskforge", "--config", "/etc/taskforge/prod.toml", "list"];
        let cli = Cli::parse_from(args);
        assert_eq!(cli.config, "/etc/taskforge/prod.toml");
        assert!(matches!(cli.command, Commands::List));
    }

    #[test]
    fn test_default_config_path() {
        let args = vec!["taskforge", "list"];
        let cli = Cli::parse_from(args);
        assert_eq!(cli.config, "config/tasks.toml");
    }
}