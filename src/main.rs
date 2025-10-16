use anyhow::{Context, Result};
use clap::Parser;
use rayon::prelude::*;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use walkdir::WalkDir;

/// CLI arguments for wav-files-denoise.
#[derive(Parser, Debug)]
#[command(author, version, about = "Recursively denoise WAV files using an external API", long_about = None)]
struct Args {
    /// Input directory containing WAV files (processed recursively)
    input_dir: PathBuf,

    /// Output directory for denoised files
    output_dir: PathBuf,

    /// Comma-separated list of API server addresses
    #[arg(long, value_delimiter = ',')]
    addr_api: Vec<String>,

    /// Model to use for denoising
    #[arg(long)]
    model: Option<String>,
}

#[derive(Serialize)]
struct DenoiseRequestBody {
    filename: String,
    filename_denoised: String,
    model: Option<String>,
}

/// Validates a WAV file matches the expected format: mono, 16-bit PCM, 16kHz sample rate.
fn validate_wav(path: &Path) -> Result<bool> {
    let reader = hound::WavReader::open(path)
        .with_context(|| format!("Failed to open WAV file: {}", path.display()))?;

    let spec = reader.spec();
    Ok(spec.channels == 1 && spec.sample_rate == 16000 && spec.bits_per_sample == 16)
}

fn main() -> Result<()> {
    let mut args = Args::parse();

    // Resolve to absolute paths to avoid ambiguity
    args.input_dir = args.input_dir.canonicalize().with_context(|| {
        format!(
            "Failed to find canonical path for input directory: {}",
            args.input_dir.display()
        )
    })?;

    // Ensure output directory exists
    std::fs::create_dir_all(&args.output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            args.output_dir.display()
        )
    })?;
    args.output_dir = args.output_dir.canonicalize().with_context(|| {
        format!(
            "Failed to find canonical path for output directory: {}",
            args.output_dir.display()
        )
    })?;

    let processed = AtomicUsize::new(0);
    let skipped = AtomicUsize::new(0);

    if args.addr_api.is_empty() {
        anyhow::bail!("At least one API address must be provided via --addr-api");
    }

    let api_endpoints = Mutex::new(args.addr_api.iter().cycle());

    let wav_files: Vec<_> = WalkDir::new(&args.input_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("wav"))
        .collect();

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(args.addr_api.len())
        .build()
        .context("Failed to create thread pool")?;

    pool.install(|| {
        wav_files.par_iter().for_each(|entry| {
            let input_path = entry.path();

            // The closure for `for_each` doesn't return a Result, so we handle errors inside.
            let process = || -> Result<()> {
                if !validate_wav(input_path)? {
                    eprintln!("Skipping invalid WAV file: {}", input_path.display());
                    skipped.fetch_add(1, Ordering::SeqCst);
                    return Ok(());
                }

                let relative = input_path.strip_prefix(&args.input_dir)?;
                let output_path = args.output_dir.join(relative);

                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent).with_context(|| {
                        format!(
                            "Failed to create output directory for: {}",
                            output_path.display()
                        )
                    })?;
                }

                let body = DenoiseRequestBody {
                    filename: input_path.to_string_lossy().to_string(),
                    filename_denoised: output_path.to_string_lossy().to_string(),
                    model: args.model.clone(),
                };

                let api_addr = api_endpoints.lock().unwrap().next().unwrap();

                let resp = ureq::post(api_addr).send_json(&body)?;

                if resp.status() == 200 {
                    processed.fetch_add(1, Ordering::SeqCst);
                } else {
                    eprintln!(
                        "Denoising failed for {}: API returned status {}",
                        input_path.display(),
                        resp.status()
                    );
                    skipped.fetch_add(1, Ordering::SeqCst);
                }
                Ok(())
            };

            if let Err(e) = process() {
                eprintln!("Error processing {}: {:?}", input_path.display(), e);
                skipped.fetch_add(1, Ordering::SeqCst);
            }
        });
    });

    println!(
        "Denoising complete: {} files processed, {} skipped.",
        processed.load(Ordering::SeqCst),
        skipped.load(Ordering::SeqCst)
    );
    Ok(())
}
