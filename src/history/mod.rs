pub mod query;
pub mod store;



// Réexporte les fonctions principales pour un accès pratique depuis main.rs
pub use store::start_history_writer;
pub use query::{last_execution, success_rate, list_known_tasks};
