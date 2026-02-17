use anyhow::Result;

use tesseras::node::Node;

pub fn run_with_node(node: &Node) -> Result<()> {
    let tesseras = node.list_tesseras()?;

    if tesseras.is_empty() {
        eprintln!("No tesseras found.");
        return Ok(());
    }

    for t in &tesseras {
        let name = t.name.as_deref().unwrap_or("(unnamed)");
        let files = t.memories.len();
        let size: u64 = t.memories.iter().map(|m| m.size).sum();
        let vis = &t.visibility;
        println!(
            "{}  {}  {} file(s)  {}  {}",
            &t.hash.to_string()[..12],
            name,
            files,
            format_size(size),
            vis,
        );
    }
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
