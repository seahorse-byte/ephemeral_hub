// The prelude brings all the essential Dioxus items into scope.
use dioxus::prelude::*;
use dioxus_router::prelude::*;
use futures_util::stream::StreamExt;
use gloo_timers::future::sleep;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// Define the routes for our application.
#[derive(Routable, Clone, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[route("/")]
    Home {},
    #[route("/s/:id")]
    Space { id: String },
}

/// The main application component that sets up the router.
#[allow(non_snake_case)]
fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

// --- Page Components ---

/// The component for the home page.
#[allow(non_snake_case)]
fn Home() -> Element {
    let navigator = use_navigator();
    // A coroutine is an async task managed by the Dioxus scheduler.
    // This is the correct way to handle async operations that trigger UI updates.
    let coroutine = use_coroutine(move |mut rx: UnboundedReceiver<()>| {
        // The coroutine needs its own clone of the navigator.
        let navigator = navigator.clone();
        async move {
            // Wait for a message from the `onclick` handler.
            while rx.next().await.is_some() {
                let api_url = "http://127.0.0.1:3000/api/spaces";

                #[derive(Deserialize, Debug)]
                struct CreateSpaceResponse {
                    id: String,
                }

                let client = reqwest::Client::new();
                let response = client.post(api_url).send().await;

                match response {
                    Ok(resp) => {
                        if let Ok(data) = resp.json::<CreateSpaceResponse>().await {
                            // Because this is run in a coroutine, the navigator
                            // update will be correctly processed by the scheduler.
                            navigator.push(Route::Space { id: data.id });
                        } else {
                            log::error!("Failed to deserialize response from server.");
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to create space: {}", e);
                    }
                }
            }
        }
    });

    rsx! {
        div {
            style: "text-align: center; padding: 50px; font-family: sans-serif;",
            h1 { "Ephemeral Space" }
            p { "A temporary space for text, files, and brainstorming." }

            button {
                // The `onclick` handler now simply sends a message to the coroutine
                // to tell it to run.
                onclick: move |_| {
                    coroutine.send(());
                },
                "Create New Space"
            }
        }
    }
}

/// Define the properties (props) for the Space component.
#[derive(PartialEq, Props, Clone)]
struct SpaceProps {
    id: String,
}

// This struct will hold the data we fetch from the backend.
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
struct SpaceData {
    id: String,
    content: String,
    created_at: String,
    files: Vec<FileInfo>,
}

#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
struct FileInfo {
    filename: String,
    size: u64,
}

#[allow(non_snake_case)]
fn Space(props: SpaceProps) -> Element {
    let id = props.id.clone();

    // Fetch space data when component mounts
    let space_resource = use_resource(move || {
        let id = id.clone();
        async move {
            let api_url = format!("http://127.0.0.1:3000/api/spaces/{}", id);
            reqwest::get(&api_url)
                .await
                .ok()?
                .json::<SpaceData>()
                .await
                .ok()
        }
    });

    // Read the resource
    let resource_state = space_resource.read();

    rsx! {
        div {
            style: "padding: 20px; font-family: sans-serif; display: flex; flex-direction: column; gap: 16px;",
            h1 { "Space ID: {props.id}" }
            Link { to: Route::Home {}, "Go back home" }

            {
                if let Some(inner) = &*resource_state {
                    match inner {
                        Some(data) => rsx! {
                            TextBin { data: data.clone(), space_id: props.id.clone() }
                        },
                        None => rsx! {
                            p { "Failed to load space data." }
                        }
                    }
                } else {
                    rsx! {
                        p { "Loading..." }
                    }
                }
            }
        }
    }
}

/// A sub-component specifically for the Text Bin UI.
#[derive(PartialEq, Props, Clone)]
struct TextBinProps {
    data: SpaceData,
    space_id: String,
}

#[allow(non_snake_case)]
fn TextBin(props: TextBinProps) -> Element {
    let mut text_content = use_signal(|| props.data.content.clone());
    let mut save_button_state = use_signal(|| "Save".to_string());

    let space_id = props.space_id.clone();

    let save_coroutine = use_coroutine(move |mut rx: UnboundedReceiver<String>| {
        let space_id = space_id.clone();
        async move {
            while let Some(content) = rx.next().await {
                let api_url = format!("http://127.0.0.1:3000/api/spaces/{}/text", space_id);
                let client = reqwest::Client::new();
                let res = client.put(api_url).body(content).send().await;

                if res.is_err() {
                    log::error!("Failed to save content");
                }
            }
        }
    });

    rsx! {
        div {
            h2 { "Text Bin" }
            textarea {
                style: "width: 100%; min-height: 200px; font-family: monospace; padding: 10px; border-radius: 8px; border: 1px solid #ccc;",
                value: "{text_content}",
                oninput: move |event| text_content.set(event.value()),
            }
            button {
                onclick: move |_| {
                    save_coroutine.send(text_content.read().clone());
                    save_button_state.set("Saving...".to_string());

                    let mut save_button_state = save_button_state.clone();
                    spawn(async move {
                        sleep(Duration::from_secs(2)).await;
                        save_button_state.set("Save".to_string());
                    });
                },
                "{save_button_state}"
            }
        }
    }
}

fn main() {
    // Initialize the logger for wasm environments.
    wasm_logger::init(wasm_logger::Config::default());
    // The `launch` function is brought into scope by the prelude.
    launch(App);
}
