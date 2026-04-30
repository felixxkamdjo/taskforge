// src/main.rs
//
// Point d'entrée de TaskForge.
// Deux modes :
//   - CLI  : taskforge list / status / enable / disable / run <id>
//   - Daemon : taskforge run-daemon  (boucle superviseur)

use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use clap::{Parser, Subcommand};
use chrono::Utc;

use taskforge::registry::{TaskRegistry, load_config};
use taskforge::engine::supervisor::{run_supervisor, execute_with_retry};
use taskforge::history::store::save_record;
use taskforge::history::{success_rate, total_executions};
use taskforge::cli::health::print_health_report;
use taskforge::parser::next_occurrence;

// ---------------------------------------------------------------------------
// CLI — définition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "taskforge",
    about = "Planificateur de tâches système en Rust",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Chemin vers le fichier de configuration TOML
    #[arg(short, long, default_value = "config/tasks.toml")]
    config: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Liste toutes les tâches et leur prochaine occurrence
    List,
    /// Rapport de santé : taux de succès et dernière exécution
    Status,
    /// Active une tâche désactivée
    Enable { id: String },
    /// Désactive une tâche sans la supprimer
    Disable { id: String },
    /// Force l'exécution immédiate d'une tâche (bloquant)
    Run { id: String },
    /// Lance le daemon de planification (boucle infinie)
    RunDaemon,
}

// ---------------------------------------------------------------------------
// Point d'entrée
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialisation du répertoire de logs
    let logs_dir = std::env::var("TASKFORGE_LOGS_DIR").unwrap_or_else(|_| "./logs".to_string());
    std::fs::create_dir_all(&logs_dir).expect("Impossible de créer le répertoire de logs");

    // Chargement de la configuration
    let tasks = match load_config(&cli.config) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[taskforge] Erreur de configuration : {}", e);
            std::process::exit(1);
        }
    };

    let registry = Arc::new(Mutex::new(TaskRegistry::from_tasks(tasks)));

    match cli.command {
        Commands::List       => cmd_list(&registry),
        Commands::Status     => cmd_status(&registry),
        Commands::Enable { id }  => cmd_enable(&registry, &id),
        Commands::Disable { id } => cmd_disable(&registry, &id),
        Commands::Run { id }     => cmd_run(&registry, &id).await,
        Commands::RunDaemon      => cmd_run_daemon(registry).await,
    }
}

// ---------------------------------------------------------------------------
// Commandes CLI
// ---------------------------------------------------------------------------

fn cmd_list(registry: &Arc<Mutex<TaskRegistry>>) {
    let reg = registry.lock().unwrap();
    let now = Utc::now();

    println!(
        "\n{:<20} | {:<25} | {:<22} | {:<30} | {:<8}",
        "ID", "Nom", "Planification", "Prochaine exécution", "État"
    );
    println!("{:-<20}-+-{:-<25}-+-{:-<22}-+-{:-<30}-+-{:-<8}", "", "", "", "", "");

    let mut tasks = reg.all_tasks();
    tasks.sort_by_key(|t| t.id.clone());

    for task in tasks {
        let next = next_occurrence(&task.schedule, now)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "—".to_string());

        let status = if task.enabled { "✓ actif" } else { "✗ inactif" };

        println!(
            "{:<20} | {:<25} | {:<22} | {:<30} | {:<8}",
            task.id, task.name, task.schedule.to_string(), next, status
        );
    }

    println!(
        "\n{} tâche(s) — {} active(s)\n",
        reg.len(),
        reg.active_count()
    );
}

fn cmd_status(registry: &Arc<Mutex<TaskRegistry>>) {
    let reg = registry.lock().unwrap();
    print_health_report(&reg);
}

fn cmd_enable(registry: &Arc<Mutex<TaskRegistry>>, id: &str) {
    let mut reg = registry.lock().unwrap();
    if reg.set_enabled(id, true) {
        println!("[taskforge] Tâche '{}' activée.", id);
    } else {
        eprintln!("[taskforge] Tâche '{}' introuvable.", id);
        std::process::exit(1);
    }
}

fn cmd_disable(registry: &Arc<Mutex<TaskRegistry>>, id: &str) {
    let mut reg = registry.lock().unwrap();
    if reg.set_enabled(id, false) {
        println!("[taskforge] Tâche '{}' désactivée.", id);
    } else {
        eprintln!("[taskforge] Tâche '{}' introuvable.", id);
        std::process::exit(1);
    }
}

async fn cmd_run(registry: &Arc<Mutex<TaskRegistry>>, id: &str) {
    let task = {
        let reg = registry.lock().unwrap();
        match reg.get(id) {
            Some(t) => t.clone(),
            None => {
                eprintln!("[taskforge] Tâche '{}' introuvable.", id);
                std::process::exit(1);
            }
        }
    };

    println!("[taskforge] Exécution forcée de '{}' : {}", task.id, task.command);
    let record = execute_with_retry(&task).await;

    // Affichage du résultat
    let status = if record.success { "✓ succès" } else { "✗ échec" };
    println!("[taskforge] {} (exit: {:?})", status, record.exit_code);

    if !record.stdout.is_empty() {
        println!("--- stdout ---\n{}", record.stdout.trim());
    }
    if !record.stderr.is_empty() {
        eprintln!("--- stderr ---\n{}", record.stderr.trim());
    }

    // Persistance du résultat
    if let Err(e) = save_record(&record) {
        eprintln!("[taskforge] Erreur de persistance : {}", e);
    }

    // Résumé historique post-exécution
    println!(
        "\n[historique] '{}' — {} exécution(s) | taux de succès : {:.0}%",
        id,
        total_executions(id),
        success_rate(id, 0) * 100.0
    );

    if !record.success {
        std::process::exit(1);
    }
}

async fn cmd_run_daemon(registry: Arc<Mutex<TaskRegistry>>) {
    // Canal entre le superviseur et le writer d'historique
    let (tx, rx) = mpsc::channel::<taskforge::types::ExecutionRecord>(128);
    // Affichage du résumé au démarrage
    {
        let reg = registry.lock().unwrap();
        println!(
            "\n╔══════════════════════════════════════════╗"
        );
        println!("║         TaskForge — Daemon actif         ║");
        println!(
            "╚══════════════════════════════════════════╝"
        );
        println!(
            " {} tâche(s) chargée(s) — {} active(s)",
            reg.len(),
            reg.active_count()
        );

        let now = Utc::now();
        let mut tasks = reg.all_tasks();
        tasks.sort_by_key(|t| t.id.clone());

        println!("\n Prochaines exécutions :");
        for task in tasks.iter().filter(|t| t.enabled) {
            if let Some(next) = next_occurrence(&task.schedule, now) {
                println!(
                    "   {:20} → {}",
                    task.id,
                    next.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }
        }
        println!();
    }

    // Writer d'historique : consomme le canal et persiste chaque record
    tokio::spawn(async move {
        let mut rx = rx;
        while let Some(record) = rx.recv().await {
            let status = if record.success { "✓" } else { "✗" };
            println!(
                "[{}] {} '{}' (exit: {:?})",
                record.start_time.format("%H:%M:%S"),
                status,
                record.task_id,
                record.exit_code,
            );
            if let Err(e) = save_record(&record) {
                eprintln!("[taskforge] Erreur persistance '{}' : {}", record.task_id, e);
            }
        }
    });

    // Boucle principale du superviseur (infinie)
    println!("[taskforge] Superviseur démarré — tick toutes les 60s\n");
    run_supervisor(registry, tx).await;
}