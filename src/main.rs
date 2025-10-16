use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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

#[derive(Deserialize)]
struct DenoiseResponseBody {
    filename_denoised: String,
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

    let mut processed = 0;
    let mut skipped = 0;

    if args.addr_api.is_empty() {
        anyhow::bail!("At least one API address must be provided via --addr-api");
    }

    let mut api_endpoints = args.addr_api.iter().cycle();

    for entry in WalkDir::new(&args.input_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("wav"))
    {
        let input_path = entry.path().to_path_buf();

        // Validate WAV format
        if !validate_wav(&input_path)? {
            eprintln!("Skipping invalid WAV file: {}", input_path.display());
            skipped += 1;
            continue;
        }

        // Compute relative path for output
        let relative = input_path.strip_prefix(&args.input_dir)?;

        let output_path = args.output_dir.join(relative);

        // Ensure output parent directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create output directory for: {}",
                    output_path.display()
                )
            })?;
        }

        // Send a command to API to enhance this file
        let body = DenoiseRequestBody {
            filename: input_path.to_string_lossy().to_string(),
            filename_denoised: output_path.to_string_lossy().to_string(),
            model: args.model.clone(),
        };

        let api_addr = api_endpoints.next().unwrap(); // Will not panic as we check for empty list

        // Requires the `json` feature enabled.
        let recv_body = ureq::post(api_addr)
            .send_json(&body)?
            .body_mut()
            .read_json::<DenoiseResponseBody>()?;

        if recv_body.filename_denoised.is_empty() {
            eprintln!("Denoising failed for {}", input_path.display(),);
            skipped += 1;
            continue;
        }

        processed += 1;
    }

    println!(
        "Denoising complete: {} files processed, {} skipped.",
        processed, skipped
    );
    Ok(())
}
