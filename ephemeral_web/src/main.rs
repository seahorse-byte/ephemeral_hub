// The prelude brings all the essential Dioxus items into scope.
use dioxus::prelude::*;
use dioxus_router::prelude::*;
use futures_util::stream::StreamExt;
use gloo_timers::future::sleep;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use web_sys::{File, FormData};

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
                            FileDrop { space_id: props.id.clone(), files: data.files.clone(), space_resource: space_resource.clone() }
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

/// A sub-component for the File Drop UI.
/// A sub-component for the File Drop UI.
#[derive(PartialEq, Props, Clone)]
struct FileDropProps {
    space_id: String,
    files: Vec<FileInfo>,
    space_resource: Resource<Option<SpaceData>>,
}

#[allow(non_snake_case)]
fn FileDrop(props: FileDropProps) -> Element {
    let is_uploading = use_signal(|| false);

    // The coroutine now expects a Vec containing the filename and its bytes.
    let upload_coroutine =
        use_coroutine(move |mut rx: UnboundedReceiver<Vec<(String, Vec<u8>)>>| {
            let space_id = props.space_id.clone();
            let mut space_resource = props.space_resource.clone();
            let mut is_uploading = is_uploading.clone();
            async move {
                while let Some(files_with_data) = rx.next().await {
                    is_uploading.set(true);
                    let mut form = multipart::Form::new();
                    for (filename, file_bytes) in files_with_data {
                        let part = multipart::Part::bytes(file_bytes).file_name(filename);
                        form = form.part("file", part);
                    }

                    let client = reqwest::Client::new();
                    let api_url = format!("http://127.0.0.1:3000/api/spaces/{}/files", space_id);

                    let res = client.post(api_url).multipart(form).send().await;

                    if res.is_ok() {
                        space_resource.restart();
                    } else {
                        log::error!("Failed to upload files: {:?}", res.err());
                    }
                    is_uploading.set(false);
                }
            }
        });

    rsx! {
        div {
            h2 { "File Drop" }
            p { "Upload files to share them temporarily." }

            input {
                r#type: "file",
                multiple: true,
                id: "file-upload",
                class: "hidden",
                onchange: move |evt| {
                    // Spawn a task to handle the async file reading.
                    spawn({
                        let upload_coroutine = upload_coroutine.clone();
                        async move {
                            // Use `.files()` to get the file engine.
                            if let Some(file_engine) = evt.files() {
                                // Get the list of file names.
                                let files = file_engine.files();
                                let mut files_with_data = Vec::new();
                                // Iterate over the file names and read each one.
                                for file_name in &files {
                                    if let Some(file_bytes) = file_engine.read_file(file_name).await {
                                        files_with_data.push((file_name.clone(), file_bytes));
                                    }
                                }
                                if !files_with_data.is_empty() {
                                    upload_coroutine.send(files_with_data);
                                }
                            }
                        }
                    });
                }
            }

            // The button that triggers the file input.
            label {
                r#for: "file-upload",
                class: "cursor-pointer bg-blue-500 text-white py-2 px-4 rounded",
                if is_uploading() {
                    "Uploading..."
                } else {
                    "Upload Files"
                }
            }

            // List of existing files.
            ul {
                class: "list-disc pl-5 mt-4",
                for file in props.files.iter() {
                    li { "{file.filename} ({file.size} bytes)" }
                }
            }
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    launch(App);
}
