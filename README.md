# wav-files-denoise-api

A command-line tool for recursively denoising WAV audio files using an external API server. It validates files against a specific format (mono channel, 16-bit PCM, 16kHz sample rate) before processing, preserves the input directory structure in the output, and provides summary statistics on completion.

## Features

- **Recursive Scanning**: Walks the input directory tree to find all `.wav` files.
- **Format Validation**: Ensures WAV files meet the required specs using the `hound` crate.
- **API Integration**: Sends JSON requests to an external denoising API via `ureq` and handles responses.
- **Robust Error Handling**: Uses `anyhow` for contextual error propagation and logging.
- **Directory Preservation**: Mirrors the input folder structure in the output directory.
- **CLI-Friendly**: Built with `clap` for intuitive argument parsing and help output.

## Prerequisites

- Rust 1.70+ (stable channel)
- An external API server running at the specified address, accepting POST requests with JSON payloads for denoising (expected request body: `{ "filename": String, "filename_denoised": String }`; response: `{ "filename_denoised": String }`).

## Installation

### From Source

1. Clone the repository:
   ```bash
   git clone https://github.com/RustedBytes/wav-files-denoise-api.git
   cd wav-files-denoise-api
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

   The binary will be available at `target/release/wav-files-denoise-api`.

### As a Cargo Dependency (Optional)

Add to your `Cargo.toml`:
```toml
[[bin]]
name = "wav-files-denoise-api"
path = "src/main.rs"
```

Then build with `cargo build --release`.

## Usage

Run the tool with required positional arguments for input and output directories, and the API address via a flag.

```bash
wav-files-denoise-api /path/to/input/dir /path/to/output/dir --addr-api http://localhost:8080/denoise
```

### Arguments

- `INPUT_DIR`: Path to the directory containing WAV files (scanned recursively).
- `OUTPUT_DIR`: Path to the directory where denoised files will be saved (created if it doesn't exist).
- `--addr-api <ADDR_API>`: The URL endpoint of the denoising API server (required).

### Example

Process all valid WAV files in `./raw_audio/` and save results to `./processed_audio/` using a local API:

```bash
./target/release/wav-files-denoise-api ./raw_audio ./processed_audio --addr-api http://127.0.0.1:3000/api/denoise
```

Output:
```
Skipping invalid WAV file: ./raw_audio/subdir/invalid.wav
Denoising failed for ./raw_audio/another.wav
Denoising complete: 5 files processed, 2 skipped.
```

## Testing

The project includes unit tests for WAV validation and error scenarios. Run them with:

```bash
cargo test
```

## Dependencies

This tool relies on the following crates (as defined in `Cargo.toml`):

| Crate | Purpose | Version Constraint |
|-------|---------|--------------------|
| `anyhow` | Contextual error handling | `^1.0` |
| `clap` | CLI argument parsing | `{ version = "^4.0", features = ["derive"] }` |
| `hound` | WAV file reading and validation | `^3.5` |
| `serde` | JSON serialization/deserialization | `{ version = "^1.0", features = ["derive"] }` |
| `ureq` | HTTP client for API requests | `{ version = "^2.0", features = ["json"] }` |
| `walkdir` | Recursive directory traversal | `^2.3` |

No additional runtime dependencies beyond the Rust standard library.

## Contributing

1. Fork the repo.
2. Create a feature branch (`git checkout -b feature/my-feature`).
3. Commit changes (`git commit -am 'Add my feature'`).
4. Push to the branch (`git push origin feature/my-feature`).
5. Open a Pull Request.

Please ensure code is formatted with `cargo fmt` and tests pass before submitting.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
