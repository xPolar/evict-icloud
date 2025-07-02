use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use clap::Parser;
use rayon::prelude::*;
use walkdir::WalkDir;

/// Evict downloaded iCloud files inside a directory tree using `brctl evict`.
#[derive(Parser, Debug)]
#[command(name = "evict-icloud", version, about)]
struct Cli {
    /// Target directory to process
    directory: PathBuf,

    /// Maximum number of concurrent evictions (defaults to logical CPU count)
    #[arg(short, long)]
    concurrency: Option<usize>,

    /// Print the file paths that would be evicted without executing `brctl evict`
    #[arg(short, long)]
    dry_run: bool,
}


fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

fn print_summary(stats: &Arc<(AtomicUsize, AtomicUsize, AtomicUsize, AtomicU64, AtomicU64, AtomicU64)>) {
    let attempted = stats.0.load(Ordering::Relaxed);
    let successful = stats.1.load(Ordering::Relaxed);
    let failed = stats.2.load(Ordering::Relaxed);
    let attempted_bytes = stats.3.load(Ordering::Relaxed);
    let successful_bytes = stats.4.load(Ordering::Relaxed);
    let failed_bytes = stats.5.load(Ordering::Relaxed);

    println!("\n=== Summary ===");
    println!("Files attempted: {} ({})", attempted, format_bytes(attempted_bytes));
    println!("Files successful: {} ({})", successful, format_bytes(successful_bytes));
    println!("Files failed: {} ({})", failed, format_bytes(failed_bytes));
    println!("Eviction complete.");
}

fn main() {
    // Enable standard backtrace via environment variable if desired.

    let cli = Cli::parse();

    let concurrency = cli.concurrency.unwrap_or_else(num_cpus::get);

    // Collect file paths first so rayon can split work among threads
    let files: Vec<PathBuf> = WalkDir::new(&cli.directory)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect();

    if files.is_empty() {
        eprintln!("No files found in {:?}", cli.directory);
        return;
    }

    let stats = Arc::new((
        AtomicUsize::new(0), // attempted
        AtomicUsize::new(0), // successful
        AtomicUsize::new(0), // failed
        AtomicU64::new(0),   // attempted bytes
        AtomicU64::new(0),   // successful bytes
        AtomicU64::new(0),   // failed bytes
    ));

    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let stats_clone = Arc::clone(&stats);
    let shutdown_clone = Arc::clone(&shutdown_flag);

    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, stopping gracefully...");
        shutdown_clone.store(true, Ordering::Relaxed);
        print_summary(&stats_clone);
        std::process::exit(0);
    }).expect("Error setting Ctrl+C handler");

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(concurrency)
        .build()
        .expect("Failed to build thread pool");

    pool.install(|| {
        files.par_iter().for_each(|file_path| {
            if shutdown_flag.load(Ordering::Relaxed) {
                return;
            }

            stats.0.fetch_add(1, Ordering::Relaxed);

            // Get file size before processing
            let file_size = match std::fs::metadata(file_path) {
                Ok(metadata) => metadata.len(),
                Err(err) => {
                    eprintln!("Failed to get metadata for {}: {}", file_path.display(), err);
                    stats.2.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            };

            stats.3.fetch_add(file_size, Ordering::Relaxed);

            if cli.dry_run {
                println!("[dry-run] Would evict: {} ({})", file_path.display(), format_bytes(file_size));
                stats.1.fetch_add(1, Ordering::Relaxed);
                stats.4.fetch_add(file_size, Ordering::Relaxed);
                return;
            }

            match Command::new("brctl")
                .args(["evict", file_path.to_str().unwrap()])
                .status()
            {
                Ok(status) if status.success() => {
                    println!("evicted content of '{}' ({})", file_path.display(), format_bytes(file_size));
                    stats.1.fetch_add(1, Ordering::Relaxed);
                    stats.4.fetch_add(file_size, Ordering::Relaxed);
                }
                Ok(status) => {
                    eprintln!(
                        "Failed evicting {} ({}) - brctl command failed (exit code: {:?})",
                        file_path.display(),
                        format_bytes(file_size),
                        status.code()
                    );
                    stats.2.fetch_add(1, Ordering::Relaxed);
                    stats.5.fetch_add(file_size, Ordering::Relaxed);
                }
                Err(err) => {
                    eprintln!("Failed evicting {} ({}) - brctl command error: {}", file_path.display(), format_bytes(file_size), err);
                    stats.2.fetch_add(1, Ordering::Relaxed);
                    stats.5.fetch_add(file_size, Ordering::Relaxed);
                }
            }
        });
    });

    print_summary(&stats);
} 