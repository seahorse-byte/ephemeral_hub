use clap::{Parser, Subcommand};
use serde::Deserialize;
use spinners::{Spinner, Spinners};
use std::io::{self, Read}; // Import Read for stdin
use std::time::Duration;

// The base URL for our backend API.
const API_BASE_URL: &str = "http://127.0.0.1:3000";

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
}

// This struct is used to deserialize the JSON response from the backend.
#[derive(Deserialize, Debug)]
struct CreateSpaceResponse {
    id: String,
    url: String,
    text_url: String,
    expires_at: String,
}

// Helper function to extract the space ID from a URL.
fn extract_id_from_url(url: &str) -> Option<String> {
    url.split('/').last().map(|s| s.to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create => {
            let mut sp = Spinner::new(Spinners::Dots9, "Creating a new space...".into());

            let client = reqwest::Client::new();
            let api_url = format!("{}/api/spaces", API_BASE_URL);

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
            let api_url = format!("{}/api/spaces/{}/text", API_BASE_URL, space_id);

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
    }

    Ok(())
}
