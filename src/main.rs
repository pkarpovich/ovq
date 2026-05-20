mod frontmatter;
mod output;
mod query;
mod values;
mod vault;

use clap::Parser;
use std::path::{Path, PathBuf};
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

    #[arg(
        long,
        help = "Output selected frontmatter fields as TSV (comma-separated field names)"
    )]
    fields: Option<String>,

    #[arg(long, help = "Output as JSON")]
    json: bool,

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

    if cli.values.is_some() && cli.fields.is_some() {
        eprintln!("Error: --fields cannot combine with --values");
        return ExitCode::from(2);
    }

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
        return run_values_mode(&frontmatters, &property, cli.count, cli.json);
    }

    let Some(query_str) = cli.query else {
        eprintln!("Error: No query provided");
        return ExitCode::from(2);
    };

    run_query_mode(
        &frontmatters,
        &query_str,
        &vault_path,
        cli.fields.as_deref(),
        cli.json,
    )
}

fn run_values_mode(
    frontmatters: &[(PathBuf, serde_yaml::Value)],
    property: &str,
    show_count: bool,
    json: bool,
) -> ExitCode {
    let data: Vec<(String, serde_yaml::Value)> = frontmatters
        .iter()
        .map(|(p, fm)| (p.display().to_string(), fm.clone()))
        .collect();

    let counts = values::collect_values(&data, property);

    if json {
        let total = counts.len();
        println!("{}", output::format_json_values(&counts, show_count));
        return ExitCode::from(exit_for_values_run(total));
    }

    let count_total = counts.len();
    let lines = values::format_values(counts, show_count);
    for line in lines {
        println!("{}", line);
    }

    ExitCode::from(exit_for_values_run(count_total))
}

fn run_query_mode(
    frontmatters: &[(PathBuf, serde_yaml::Value)],
    query_str: &str,
    vault_path: &Path,
    fields_spec: Option<&str>,
    json: bool,
) -> ExitCode {
    let expr = match query::parse(query_str) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Query error: {}", e);
            return ExitCode::from(2);
        }
    };

    let matches: Vec<(PathBuf, serde_yaml::Value)> = frontmatters
        .iter()
        .filter(|(_, fm)| query::evaluate(&expr, fm))
        .map(|(p, fm)| (p.clone(), fm.clone()))
        .collect();

    let parsed_fields: Vec<String> = match fields_spec {
        Some(spec) => output::parse_fields(spec),
        None => Vec::new(),
    };

    if json {
        let field_refs: Vec<&str> = parsed_fields.iter().map(String::as_str).collect();
        let fields_opt = fields_spec.map(|_| field_refs.as_slice());
        println!(
            "{}",
            output::format_json_query(&matches, vault_path, fields_opt)
        );
    } else if fields_spec.is_some() {
        let field_refs: Vec<&str> = parsed_fields.iter().map(String::as_str).collect();
        for (path, fm) in &matches {
            println!(
                "{}",
                output::format_tsv(path, vault_path, fm, &field_refs)
            );
        }
    } else {
        for (path, _) in &matches {
            println!("{}", output::format_path(path, vault_path));
        }
    }

    ExitCode::from(exit_for_query_run(matches.len()))
}

fn exit_for_query_run(_matched_count: usize) -> u8 {
    0
}

fn exit_for_values_run(_count: usize) -> u8 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_for_query_run_zero_matches_is_zero() {
        assert_eq!(exit_for_query_run(0), 0);
    }

    #[test]
    fn exit_for_query_run_any_count_is_zero() {
        assert_eq!(exit_for_query_run(1), 0);
        assert_eq!(exit_for_query_run(42), 0);
        assert_eq!(exit_for_query_run(10_000), 0);
    }

    #[test]
    fn exit_for_values_run_zero_is_zero() {
        assert_eq!(exit_for_values_run(0), 0);
    }

    #[test]
    fn exit_for_values_run_any_count_is_zero() {
        assert_eq!(exit_for_values_run(1), 0);
        assert_eq!(exit_for_values_run(99), 0);
    }
}
