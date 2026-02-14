use anyhow::Result;

use super::create::build_service;
use super::init::expand_tilde;

pub async fn run(data_dir: &str) -> Result<()> {
    let base = expand_tilde(data_dir);
    let service = build_service(&base)?;
    let tesseras = service.list().await?;

    if tesseras.is_empty() {
        println!("No tesseras found.");
        return Ok(());
    }

    let mut table = comfy_table::Table::new();
    table.set_header(vec!["Hash", "Created", "Memories", "Size", "Visibility"]);
    for t in &tesseras {
        let hash_short = &t.hash.to_string()[..16];
        let date = t.created_at.format("%Y-%m-%d").to_string();
        let size = format_size(t.size_bytes);
        table.add_row(vec![
            hash_short.to_string(),
            date,
            t.memory_count.to_string(),
            size,
            t.visibility.clone(),
        ]);
    }
    println!("{table}");
    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
