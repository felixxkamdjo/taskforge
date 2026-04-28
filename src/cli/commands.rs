use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "taskforge")]
#[command(about = "Planificateur de tâches systèmes en Rust", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    // Liste toutes les tâches enregistrées
    List,
    // Force l'exécution immédiate d'une tâche
    Run { name: String },
    // Active une tâche désactivée
    Enable { name: String },
    // Désactive une tâche sans la supprimer
    Disable { name: String },
    // Affiche le statut en temps réel et le rapport de santé
    Status,
}