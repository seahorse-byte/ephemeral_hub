use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use comfy_table::Table;
use serde::Deserialize;
use spinners::{Spinner, Spinners};
use std::env;
use std::io::{self, Read};
use std::path::PathBuf;
use tokio::fs;

const EPHEMERAL_BANNER: &str = r#"
                .==-.                   .-==.
                 \()8`-._  `.   .'  _.-'8()/
                 (88"   ::.  \./  .::   "88)
                  \_.'`-::::.(#).::::-'`._/
                    `._... .q(_)p. ..._.'
                      ""-..-'|=|`-..-""
                      .""' .'|=|`. `"".
                    ,':8(o)./|=|\.(o)8:`.
                   (O :8 ::/ \_/ \:: 8: O)
                    \O `::/       \::' O/
                     ""--'         `--""
"#;
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new ephemeral hub.
    Create,
    /// Pipe text into a hub's text bin.
    Pipe {
        /// The URL of the hub.
        url: String,
    },
    /// Upload a file to a hub.
    Upload {
        /// Path to the file to upload.
        file_path: PathBuf,
        /// The URL of the hub.
        url: String,
    },
    /// Download all content from a hub as a zip file.
    Get {
        /// The URL of the hub.
        url: String,
    },
}

#[derive(Deserialize, Debug)]
struct CreateHubResponse {
    id: String,
    // We no longer need the URLs from the server, but we keep the struct
    // the same to match the backend's JSON response.
    url: String,      // not used within the CLI
    text_url: String, // not used within the CLI
    expires_at: DateTime<Utc>,
}

// Gets the API base URL from an environment variable, with a production default.
fn get_api_base_url() -> String {
    env::var("EPHEMERAL_API_URL").unwrap_or_else(|_| "https://api.ephemeral-hub.com".to_string())
}

// Extracts the hub ID from various possible URL formats.
fn extract_hub_id(url: &str) -> Option<String> {
    let parts: Vec<&str> = url.split('/').collect();
    // Handles URLs like .../hubs/{id}, .../hubs/{id}/text, .../hubs/{id}/files, etc.
    if let Some(hubs_index) = parts.iter().position(|&p| p == "hubs") {
        if hubs_index + 1 < parts.len() {
            return Some(parts[hubs_index + 1].to_string());
        }
    }
    None
}

#[tokio::main]
async fn main() {
    println!("{}", EPHEMERAL_BANNER);
    let cli = Cli::parse();
    let api_base_url = get_api_base_url();

    // Configure a reqwest client that follows redirects.
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::default())
        .build()
        .unwrap();

    match cli.command {
        Commands::Create => {
            let mut sp = Spinner::new(Spinners::Dots9, "Creating a new hub...".into());
            let api_url = format!("{}/api/hubs", api_base_url);

            let response = client.post(&api_url).send().await;

            sp.stop();

            match response {
                Ok(res) => {
                    if res.status().is_success() {
                        println!("\n✓ Hub created successfully!");
                        match res.json::<CreateHubResponse>().await {
                            Ok(hub) => {
                                // Construct the URL on the client-side to ensure it's correct.
                                let correct_api_url =
                                    format!("{}/api/hubs/{}", api_base_url, hub.id);

                                let mut table = Table::new();
                                table.set_header(vec!["Attribute", "Value"]);
                                table.add_row(vec!["Hub ID", &hub.id]);
                                table.add_row(vec!["API URL", &correct_api_url]);
                                table
                                    .add_row(vec!["Expires At (UTC)", &hub.expires_at.to_string()]);
                                println!("{table}");
                            }
                            Err(_) => {
                                println!("Error: Failed to parse server response.");
                            }
                        }
                    } else {
                        println!("Error: Failed to create hub (Status: {})", res.status());
                    }
                }
                Err(e) => {
                    println!("Error: Could not connect to the server: {}", e);
                }
            }
        }
        Commands::Pipe { url } => {
            if let Some(hub_id) = extract_hub_id(&url) {
                let mut sp = Spinner::new(Spinners::Dots9, "Piping content...".into());
                let api_url = format!("{}/api/hubs/{}/text", api_base_url, hub_id);

                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).unwrap();

                let response = client.put(&api_url).body(buffer).send().await;
                sp.stop();

                match response {
                    Ok(res) if res.status().is_success() => {
                        println!("\n✓ Content piped successfully!");
                    }
                    Ok(res) => {
                        println!("\nError: Failed to pipe content (Status: {})", res.status());
                    }
                    Err(e) => {
                        println!("\nError: Could not connect to the server: {}", e);
                    }
                }
            } else {
                println!("Error: Invalid URL format provided.");
            }
        }
        Commands::Upload { file_path, url } => {
            if !file_path.exists() {
                println!("Error: File not found at '{}'", file_path.display());
                return;
            }

            if let Some(hub_id) = extract_hub_id(&url) {
                let mut sp = Spinner::new(Spinners::Dots9, "Uploading file...".into());
                let api_url = format!("{}/api/hubs/{}/files", api_base_url, hub_id);

                let file_name = file_path.file_name().unwrap().to_str().unwrap().to_string();
                let file_bytes = fs::read(&file_path).await.unwrap();

                let part = reqwest::multipart::Part::bytes(file_bytes).file_name(file_name);
                let form = reqwest::multipart::Form::new().part("file", part);

                let response = client.post(&api_url).multipart(form).send().await;
                sp.stop();

                match response {
                    Ok(res) if res.status().is_success() => {
                        println!("\n✓ File uploaded successfully!");
                    }
                    Ok(res) => {
                        println!("\nError: Failed to upload file (Status: {})", res.status());
                    }
                    Err(e) => {
                        println!("\nError: Could not connect to the server: {}", e);
                    }
                }
            } else {
                println!("Error: Invalid URL format provided.");
            }
        }
        Commands::Get { url } => {
            if let Some(hub_id) = extract_hub_id(&url) {
                let mut sp = Spinner::new(Spinners::Dots9, "Downloading hub content...".into());
                let api_url = format!("{}/api/hubs/{}/download", api_base_url, hub_id);

                let response = client.get(&api_url).send().await;
                sp.stop();

                match response {
                    Ok(res) if res.status().is_success() => {
                        let file_name = format!("ephemeral_hub_{}.zip", hub_id);
                        let bytes = res.bytes().await.unwrap();
                        fs::write(&file_name, bytes).await.unwrap();
                        println!("\n✓ Hub content downloaded to '{}'", file_name);
                    }
                    Ok(res) => {
                        println!("\nError: Failed to download hub (Status: {})", res.status());
                    }
                    Err(e) => {
                        println!("\nError: Could not connect to the server: {}", e);
                    }
                }
            } else {
                println!("Error: Invalid URL format provided.");
            }
        }
    }
}
