mod frontmatter;
mod query;
mod values;
mod vault;

use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "ovq", about = "Query Obsidian vault files by frontmatter properties")]
struct Cli {
    #[arg(long, env = "OVQ_VAULT")]
    vault: Option<PathBuf>,

    #[arg(long, help = "List unique values for a property")]
    values: Option<String>,

    #[arg(long, help = "Show count for each value (use with --values)")]
    count: bool,

    #[arg(long, help = "Read file paths from stdin")]
    stdin: bool,

    #[arg(help = "Query in Dataview WHERE syntax")]
    query: Option<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let vault_path = match cli.vault {
        Some(p) => p,
        None => {
            eprintln!("Error: No vault path specified. Use --vault or set OVQ_VAULT");
            return ExitCode::from(2);
        }
    };

    let files = if cli.stdin {
        vault::read_paths_from_stdin()
    } else {
        vault::collect_markdown_files(&vault_path)
    };

    let frontmatters: Vec<(PathBuf, serde_yaml::Value)> = files
        .into_iter()
        .filter_map(|path| {
            let fm = frontmatter::parse_frontmatter(&path)?;
            Some((path, fm))
        })
        .collect();

    if let Some(property) = cli.values {
        return run_values_mode(&frontmatters, &property, cli.count);
    }

    let Some(query_str) = cli.query else {
        eprintln!("Error: No query provided");
        return ExitCode::from(2);
    };

    run_query_mode(&frontmatters, &query_str, &vault_path)
}

fn run_values_mode(
    frontmatters: &[(PathBuf, serde_yaml::Value)],
    property: &str,
    show_count: bool,
) -> ExitCode {
    let data: Vec<(String, serde_yaml::Value)> = frontmatters
        .iter()
        .map(|(p, fm)| (p.display().to_string(), fm.clone()))
        .collect();

    let counts = values::collect_values(&data, property);

    if counts.is_empty() {
        return ExitCode::from(1);
    }

    let lines = values::format_values(counts, show_count);
    for line in lines {
        println!("{}", line);
    }

    ExitCode::from(0)
}

fn run_query_mode(
    frontmatters: &[(PathBuf, serde_yaml::Value)],
    query_str: &str,
    vault_path: &PathBuf,
) -> ExitCode {
    let expr = match query::parse(query_str) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Query error: {}", e);
            return ExitCode::from(2);
        }
    };

    let mut found = false;

    for (path, fm) in frontmatters {
        if query::evaluate(&expr, fm) {
            found = true;
            let display_path = path
                .strip_prefix(vault_path)
                .unwrap_or(path)
                .display();
            println!("{}", display_path);
        }
    }

    if found {
        ExitCode::from(0)
    } else {
        ExitCode::from(1)
    }
}
