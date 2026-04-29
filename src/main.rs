use taskforge::parser::{parse_cron_expression, parse_macro, next_occurrence};
use chrono::Utc;

fn main() {
    println!("=== TaskForge - Test du Parser ===\n");

    let schedule1 = parse_cron_expression("30 8 * * 1").unwrap();
    println!("1. Expression cron: 30 8 * * 1");
    println!("   Prochaine exécution: {:?}\n", next_occurrence(&schedule1, Utc::now()));

    let schedule2 = parse_cron_expression("*/15 * * * *").unwrap();
    println!("2. Expression cron: */15 * * * *");
    println!("   Prochaine exécution: {:?}\n", next_occurrence(&schedule2, Utc::now()));

    let schedule3 = parse_macro("@daily").unwrap();
    println!("3. Macro: @daily");
    println!("   Prochaine exécution: {:?}\n", next_occurrence(&schedule3, Utc::now()));

    let schedule4 = parse_macro("@every 5m").unwrap();
    println!("4. Macro: @every 5m");
    println!("   Prochaine exécution: {:?}\n", next_occurrence(&schedule4, Utc::now()));

    let schedule5 = parse_macro("@every 2h").unwrap();
    println!("5. Macro: @every 2h");
    println!("   Prochaine exécution: {:?}\n", next_occurrence(&schedule5, Utc::now()));

    match parse_macro("@invalid") {
        Some(_) => println!("6. Macro @invalid: parsée (inattendu)"),
        None => println!("6. Macro @invalid: rejetée (comportement attendu)"),
    }
}