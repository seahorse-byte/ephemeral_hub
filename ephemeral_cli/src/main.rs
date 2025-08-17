use clap::{Parser, Subcommand};
use reqwest::multipart;
use serde::Deserialize;
use spinners::{Spinner, Spinners};
use std::env; // To read environment variables
use std::io::{self, Read};
use std::path::PathBuf; // For handling file paths
use tokio::fs; // For async file reading

// Function to get the API base URL from an environment variable.
fn get_api_base_url() -> String {
    env::var("EPHEMERAL_API_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string())
}

/// A CLI for interacting with Ephemeral Spaces.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new ephemeral space.
    Create,
    /// Pipe text into a space's text bin.
    /// Example: cat log.txt | ephemeral pipe <URL>
    Pipe {
        /// The full API URL of the space (e.g., http://.../api/spaces/xyz)
        url: String,
    },
    /// Upload a file to a space.
    Upload {
        /// The path to the file to upload.
        file_path: PathBuf,
        /// The full API URL of the space.
        url: String,
    },
    /// Download all content from a space as a zip file.
    Get {
        /// The full API URL of the space.
        url: String,
    },
}

// This struct is used to deserialize the JSON response from the backend.
#[derive(Deserialize, Debug)]
struct CreateSpaceResponse {
    id: String,
    url: String,
    expires_at: String,
}

// Helper function to extract the space ID from a URL.
fn extract_id_from_url(url: &str) -> Option<String> {
    // This logic is a bit naive and assumes the ID is the last part of the URL.
    // A more robust solution would use regex or a URL parsing library.
    url.split('/').last().map(|s| s.to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let api_base_url = get_api_base_url();

    match cli.command {
        Commands::Create => {
            let mut sp = Spinner::new(Spinners::Dots9, "Creating a new space...".into());

            let client = reqwest::Client::new();
            let api_url = format!("{}/api/spaces", api_base_url);

            let response = client.post(&api_url).send().await;

            sp.stop_with_message("✓ Space created successfully!".into());

            match response {
                Ok(res) => {
                    if res.status().is_success() {
                        let space_info = res.json::<CreateSpaceResponse>().await?;
                        println!("\n--- 🚀 New Ephemeral Space ---");
                        println!("ID:         {}", space_info.id);
                        println!("API URL:    {}", space_info.url);
                        println!("Expires at: {}", space_info.expires_at);
                        println!("-----------------------------");
                    } else {
                        eprintln!("Error: Failed to create space (Status: {})", res.status());
                    }
                }
                Err(e) => {
                    eprintln!("Error: Could not connect to the server: {}", e);
                }
            }
        }
        Commands::Pipe { url } => {
            let mut sp = Spinner::new(Spinners::Dots9, "Piping text to space...".into());

            // Read all content from standard input.
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;

            if buffer.trim().is_empty() {
                sp.stop_with_message("✗ No text provided to pipe.".into());
                return Ok(());
            }

            // Extract the ID from the provided URL.
            let space_id = match extract_id_from_url(&url) {
                Some(id) => id,
                None => {
                    sp.stop_with_message("✗ Invalid space URL provided.".into());
                    return Ok(());
                }
            };

            let client = reqwest::Client::new();
            let api_url = format!("{}/api/spaces/{}/text", api_base_url, space_id);

            let response = client.put(&api_url).body(buffer).send().await;

            match response {
                Ok(res) => {
                    if res.status().is_success() {
                        sp.stop_with_message("✓ Text piped successfully!".into());
                    } else {
                        sp.stop_with_message(
                            format!("✗ Error: Failed to pipe text (Status: {})", res.status())
                                .into(),
                        );
                    }
                }
                Err(e) => {
                    sp.stop_with_message(
                        format!("✗ Error: Could not connect to the server: {}", e).into(),
                    );
                }
            }
        }
        Commands::Upload { file_path, url } => {
            if !file_path.exists() {
                eprintln!("✗ Error: File not found at '{}'", file_path.display());
                return Ok(());
            }

            let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
            let mut sp = Spinner::new(
                Spinners::Dots9,
                format!("Uploading '{}'...", file_name).into(),
            );

            let space_id = match extract_id_from_url(&url) {
                Some(id) => id,
                None => {
                    sp.stop_with_message("✗ Invalid space URL provided.".into());
                    return Ok(());
                }
            };

            // Read the file's contents asynchronously.
            let file_bytes = fs::read(&file_path).await?;
            let part = multipart::Part::bytes(file_bytes).file_name(file_name);
            let form = multipart::Form::new().part("file", part);

            let client = reqwest::Client::new();
            let api_url = format!("{}/api/spaces/{}/files", api_base_url, space_id);

            let response = client.post(&api_url).multipart(form).send().await;

            match response {
                Ok(res) => {
                    if res.status().is_success() {
                        sp.stop_with_message("✓ File uploaded successfully!".into());
                    } else {
                        sp.stop_with_message(
                            format!("✗ Error: Failed to upload file (Status: {})", res.status())
                                .into(),
                        );
                    }
                }
                Err(e) => {
                    sp.stop_with_message(
                        format!("✗ Error: Could not connect to the server: {}", e).into(),
                    );
                }
            }
        }
        Commands::Get { url } => {
            let mut sp = Spinner::new(Spinners::Dots9, "Downloading space content...".into());

            let space_id = match extract_id_from_url(&url) {
                Some(id) => id,
                None => {
                    sp.stop_with_message("✗ Invalid space URL provided.".into());
                    return Ok(());
                }
            };

            let client = reqwest::Client::new();
            let api_url = format!("{}/api/spaces/{}/download", api_base_url, space_id);

            let response = client.get(&api_url).send().await;

            match response {
                Ok(res) => {
                    if res.status().is_success() {
                        let file_bytes = res.bytes().await?;
                        let file_name = format!("ephemeral_space_{}.zip", space_id);
                        fs::write(&file_name, &file_bytes).await?;
                        sp.stop_with_message(
                            format!("✓ Space content saved to '{}'!", file_name).into(),
                        );
                    } else {
                        sp.stop_with_message(
                            format!("✗ Error: Failed to download (Status: {})", res.status())
                                .into(),
                        );
                    }
                }
                Err(e) => {
                    sp.stop_with_message(
                        format!("✗ Error: Could not connect to the server: {}", e).into(),
                    );
                }
            }
        }
    }

    Ok(())
}
