use chrono::{Local, TimeZone};
use clap::Parser;
use colored::*;
use git2::{BranchType, Repository};
use regex::Regex;

#[derive(Parser, Debug, Clone)]
struct Args {
    /// Show remote branches
    #[clap(short = 'r', long)]
    show_remote: bool,

    /// Show all (both local & remote) branches
    #[clap(short = 'a', long)]
    show_all: bool,

    /// Verbose output
    #[clap(short = 'v', long)]
    verbose: bool,

    // Path
    path: Option<String>,
}

fn main() -> Result<(), git2::Error> {
    let args = Args::parse();

    let path = args.path.unwrap_or_else(|| ".".to_string());

    let repo = Repository::open(path).unwrap_or_else(|err| {
        eprintln!("Failed to open repository: {}", err.message());
        std::process::exit(1);
    });

    let head_name = head_branch_name(&repo).unwrap_or_else(|| "00000000".to_string());

    let local_branches = repo.branches(Some(BranchType::Local))?;
    let remote_branches = repo.branches(Some(BranchType::Remote))?;

    let branch_iter: Box<dyn Iterator<Item = _>> = if args.show_all {
        Box::new(local_branches.chain(remote_branches))
    } else if args.show_remote {
        Box::new(remote_branches)
    } else {
        Box::new(local_branches)
    };

    let mut branches: Vec<_> = branch_iter
        .filter_map(|branch| {
            let (branch, _) = branch.ok()?;
            let name = branch.name().ok()??.to_string();
            let commit = branch.get().peel_to_commit().ok()?;
            let time = commit.time().seconds();
            Some((branch, name, time))
        })
        .collect();

    branches.sort_by(|a, b| a.2.cmp(&b.2));

    for (branch, name, time) in branches {
        println!(
            "{} {} {} {}",
            format_commit_time(time),
            format_commit_id(&branch).yellow(),
            current_mark(&name, &head_name).green(),
            format_branch_name(&name, &head_name)
        );
        if args.verbose {
            println!(
                "{:width$}{}",
                " ",
                latest_commit_message(&branch).unwrap_or_default().green(),
                width = 32
            );
        }
    }

    Ok(())
}

fn head_branch_name(repo: &Repository) -> Option<String> {
    repo.head().ok()?.shorthand().map(|name| name.to_string())
}

fn format_commit_id(branch: &git2::Branch) -> String {
    branch
        .get()
        .peel_to_commit()
        .ok()
        .map(|commit| commit.id().to_string()[..8].to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn format_commit_time(time: i64) -> String {
    let time = Local
        .timestamp_opt(time, 0)
        .single()
        .map(|dt| dt.format("%Y.%m.%d %H:%M").to_string())
        .unwrap_or_else(|| "?".to_string());

    format!("[{}]", time).blue().to_string()
}

fn current_mark(name: &str, head_name: &str) -> String {
    if name == head_name {
        "*".green().to_string()
    } else if is_remote(name) {
        "@".red().to_string()
    } else {
        " ".to_string()
    }
}

fn format_branch_name(name: &str, head_name: &str) -> ColoredString {
    let core_branches: Vec<&str> = vec!["master", "main", "develop", "test", "demo", "release"];
    let low_priority_branch_suffixes: Vec<&str> = vec!["old", "obs", "nouse", "merged"];

    if name == head_name {
        name.green()
    } else if is_remote(name) {
        name.red()
    } else if core_branches.contains(&name) || is_version(name) {
        name.purple()
    } else if low_priority_branch_suffixes
        .iter()
        .any(|suffix| name.ends_with(suffix))
    {
        name.blue()
    } else {
        name.white()
    }
}

fn latest_commit_message(branch: &git2::Branch) -> Option<String> {
    let commit = branch.get().peel_to_commit().ok()?;
    commit.summary().map(|s| s.to_string())
}

fn is_remote(name: &str) -> bool {
    name.starts_with("origin/")
}

fn is_version(name: &str) -> bool {
    let pattern = r"(?i)^v\d+";

    Regex::new(pattern)
        .map(|re| re.is_match(name))
        .unwrap_or(false)
}
