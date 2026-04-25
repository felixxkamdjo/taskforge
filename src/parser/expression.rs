use crate::types::{CronField, Schedule};

/// Parse une expression cron avec 5 champs : minute, heure, jour, mois, jour_semaine
pub fn parse_cron_expression(input: &str) -> Result<Schedule, String> {
    let fields: Vec<&str> = input.split_whitespace().collect();
    
    if fields.len() != 5 {
        return Err(format!("Expression cron invalide: attendu 5 champs, obtenu {}", fields.len()));
    }

    let minute = parse_field(fields[0], 0, 59)?;
    let hour = parse_field(fields[1], 0, 23)?;
    let day_of_month = parse_field(fields[2], 1, 31)?;
    let month = parse_field(fields[3], 1, 12)?;
    let day_of_week = parse_field(fields[4], 0, 7)?;

    Ok(Schedule::Cron {
        minute,
        hour,
        day_of_month,
        month,
        day_of_week,
    })
}

fn parse_field(field: &str, min: u32, max: u32) -> Result<CronField, String> {
    if field.contains('/') {
        let parts: Vec<&str> = field.split('/').collect();
        if parts.len() != 2 {
            return Err(format!("Format de step invalide: {}", field));
        }
        let step: u32 = parts[1].parse()
            .map_err(|_| format!("Step invalide: {}", parts[1]))?;
        
        if parts[0] == "*" {
            return Ok(CronField::Step(0, step));
        } else {
            if parts[0].contains('-') {
                let range: Vec<&str> = parts[0].split('-').collect();
                let start: u32 = range[0].parse().unwrap();
                // end n'est pas utilisé, mais on le parse pour validation implicite
                let _end: u32 = range[1].parse().unwrap();
                return Ok(CronField::Step(start, step));
            }
        }
    }

    if field.contains('-') {
        let parts: Vec<&str> = field.split('-').collect();
        if parts.len() != 2 {
            return Err(format!("Plage invalide: {}", field));
        }
        let start: u32 = parts[0].parse()
            .map_err(|_| format!("Valeur de début invalide: {}", parts[0]))?;
        let end: u32 = parts[1].parse()
            .map_err(|_| format!("Valeur de fin invalide: {}", parts[1]))?;
        
        validate_range(start, min, max)?;
        validate_range(end, min, max)?;
        
        return Ok(CronField::Range(start, end));
    }

    if field.contains(',') {
        let values: Result<Vec<u32>, _> = field.split(',')
            .map(|v| v.parse::<u32>())
            .collect();
        
        let values = values.map_err(|_| format!("Liste invalide: {}", field))?;
        
        for &val in &values {
            validate_range(val, min, max)?;
        }
        
        return Ok(CronField::List(values));
    }

    if field == "*" {
        return Ok(CronField::Any);
    }

    let value: u32 = field.parse()
        .map_err(|_| format!("Valeur invalide: {}", field))?;
    validate_range(value, min, max)?;
    
    Ok(CronField::Single(value))
}

fn validate_range(value: u32, min: u32, max: u32) -> Result<(), String> {
    if value < min || value > max {
        Err(format!("Valeur {} hors plage [{}, {}]", value, min, max))
    } else {
        Ok(())
    }
}