pub mod query;
pub mod store;

// Réexporte les fonctions principales pour un accès pratique depuis main.rs
pub use store::start_history_writer;
pub use query::{last_execution, success_rate, list_known_tasks, total_executions};

// Lock global partagé entre store et query pour sérialiser TOUS les tests
// du module history qui manipulent set_current_dir() (global au processus).
// Les deux sous-modules importent ce lock — avoir deux Mutex distincts ne
// suffit pas car store::TEST_LOCK et query::TEST_LOCK sont indépendants.
#[cfg(test)]
pub static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());