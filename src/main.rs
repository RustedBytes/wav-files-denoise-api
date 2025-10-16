use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// CLI arguments for wav-files-denoise.
#[derive(Parser, Debug)]
#[command(author, version, about = "Recursively denoise WAV files using nnnoiseless", long_about = None)]
struct Args {
    /// Input directory containing WAV files (processed recursively)
    input_dir: PathBuf,

    /// Output directory for denoised files
    output_dir: PathBuf,

    /// Path to nnnoiseless executable (defaults to 'nnnoiseless' in PATH)
    #[arg(long, value_name = "PATH")]
    nnnoiseless_path: Option<PathBuf>,

    /// Path to custom model file (optional; uses built-in model if omitted)
    #[arg(long, value_name = "PATH")]
    model_path: Option<PathBuf>,
}

/// Validates a WAV file matches the expected format: mono, 16-bit PCM, 16kHz sample rate.
fn validate_wav(path: &Path) -> Result<bool> {
    let reader = hound::WavReader::open(path)
        .with_context(|| format!("Failed to open WAV file: {}", path.display()))?;

    let spec = reader.spec();
    Ok(spec.channels == 1 && spec.sample_rate == 16000 && spec.bits_per_sample == 16)
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Ensure output directory exists
    std::fs::create_dir_all(&args.output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            args.output_dir.display()
        )
    })?;

    let nnnoiseless_cmd = args
        .nnnoiseless_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("nnnoiseless"));

    let mut processed = 0;
    let mut skipped = 0;

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
        let relative = input_path.strip_prefix(&args.input_dir).with_context(|| {
            format!(
                "Failed to compute relative path for: {}",
                input_path.display()
            )
        })?;

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

        // Build and execute nnnoiseless command
        let mut cmd = Command::new(&nnnoiseless_cmd);
        if let Some(ref model) = args.model_path {
            cmd.arg(format!("--model={}", model.display()));
        }
        cmd.arg(&input_path);
        cmd.arg(&output_path);

        let status = cmd.status().with_context(|| {
            format!(
                "Failed to execute nnnoiseless for: {}",
                input_path.display()
            )
        })?;

        if !status.success() {
            eprintln!(
                "Denoising failed for {}: exit code {}",
                input_path.display(),
                status
            );
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

#[cfg(test)]
mod tests {
    use super::*;
    use hound::{SampleFormat, WavSpec, WavWriter};
    use tempfile::TempDir;

    #[test]
    fn test_validate_wav_valid() -> Result<()> {
        let dir = TempDir::new()?;
        let file_path = dir.path().join("valid.wav");

        // Create a valid WAV file
        {
            let spec = WavSpec {
                channels: 1,
                sample_rate: 16000,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            };
            let mut writer = WavWriter::create(&file_path, spec)?;
            let mut sample_writer = writer.get_i16_writer(1);
            sample_writer.write_sample(0);
            sample_writer.flush()?;
        }

        assert!(validate_wav(&file_path)?);
        Ok(())
    }

    #[test]
    fn test_validate_wav_invalid_sample_rate() -> Result<()> {
        let dir = TempDir::new()?;
        let file_path = dir.path().join("invalid_rate.wav");

        // Create a WAV with wrong sample rate
        {
            let spec = WavSpec {
                channels: 1,
                sample_rate: 44100, // Invalid rate
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            };
            let mut writer = WavWriter::create(&file_path, spec)?;
            let mut sample_writer = writer.get_i16_writer(1);
            sample_writer.write_sample(0);
            sample_writer.flush()?;
        }

        assert!(!validate_wav(&file_path)?);
        Ok(())
    }

    #[test]
    fn test_validate_wav_invalid_channels() -> Result<()> {
        let dir = TempDir::new()?;
        let file_path = dir.path().join("invalid_channels.wav");

        // Create a stereo WAV
        {
            let spec = WavSpec {
                channels: 2, // Invalid channels
                sample_rate: 16000,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            };
            let mut writer = WavWriter::create(&file_path, spec).unwrap();
            let mut sample_writer = writer.get_i16_writer(2);
            sample_writer.write_sample(0);
            sample_writer.write_sample(0);
            sample_writer.flush()?;
        }

        assert!(!validate_wav(&file_path)?);
        Ok(())
    }

    #[test]
    fn test_validate_wav_invalid_bits() -> Result<()> {
        let dir = TempDir::new()?;
        let file_path = dir.path().join("invalid_bits.wav");

        // Create a 24-bit WAV
        {
            let spec = WavSpec {
                channels: 1,
                sample_rate: 16000,
                bits_per_sample: 24, // Invalid bits
                sample_format: SampleFormat::Int,
            };
            let mut writer = WavWriter::create(&file_path, spec).unwrap();
            let sample: i32 = 0;
            writer.write_sample(sample)?;
        }

        assert!(!validate_wav(&file_path)?);
        Ok(())
    }
}
