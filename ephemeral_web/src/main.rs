// The prelude brings all the essential Dioxus items into scope.
use dioxus::prelude::*;
use dioxus_router::prelude::*;
use futures_util::stream::StreamExt;
use gloo_timers::future::sleep;
use reqwest::multipart;
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

#[allow(non_snake_case)]
fn Home() -> Element {
    static LOGO: Asset = asset!("/assets/logo_no_text_1.png");

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

    let floating_shapes = (0..15).map(|i| {
        let (size, shape) = match i % 4 {
            0 => ("w-8 h-8", "rounded-lg"),
            1 => ("w-12 h-12", "rounded-full"),
            2 => ("w-6 h-6", "rounded-sm"),
            _ => ("w-10 h-10", "rounded-lg"),
        };

        let color = match i % 3 {
            0 => "bg-blue-500/20",
            1 => "bg-orange-400/20",
            _ => "bg-blue-400/30",
        };

        let delay = format!("animation-delay: {}ms;", i * 300);
        let top = format!("top: {}%;", (7 * i + 10) % 80);
        let left = format!("left: {}%;", (11 * i + 5) % 90);

        rsx! {
            div {
                class: "absolute opacity-60 animate-bounce {size} {color} {shape}",
                style: "{top} {left} {delay}"
            }
        }
    });

    let network_lines = rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            class: "absolute inset-0 w-full h-full opacity-30 pointer-events-none",
            defs {
                linearGradient {
                    id: "networkGradient",
                    x1: "0%", y1: "0%", x2: "100%", y2: "100%",
                    stop { offset: "0%", stop_color: "#9fc3fdff", stop_opacity: "0.6" }
                    stop { offset: "100%", stop_color: "#fac198ff", stop_opacity: "0.3" }
                }
            }
            line { x1: "10%", y1: "20%", x2: "30%", y2: "40%", stroke: "url(#networkGradient)", stroke_width: "1" }
            line { x1: "70%", y1: "30%", x2: "90%", y2: "50%", stroke: "url(#networkGradient)", stroke_width: "1" }
            line { x1: "20%", y1: "70%", x2: "40%", y2: "90%", stroke: "url(#networkGradient)", stroke_width: "1" }
            line { x1: "60%", y1: "10%", x2: "80%", y2: "30%", stroke: "url(#networkGradient)", stroke_width: "1" }
        }
    };

    rsx! {
        div {
            class: "bg-gradient-to-br from-slate-900 via-slate-800 to-blue-900 relative overflow-hidden",

            {network_lines}
            {floating_shapes}

            img {
                src: LOGO,
                class: "mx-auto  h-48 w-auto",
                alt: "Ephemeral Spaces Logo"
            }
            div { class: "relative z-10 flex min-h-screen px-4",
                div { class: "text-center max-w-4xl mx-auto",

                    h1 {
                        class: "text-6xl md:text-8xl font-bold mb-8 bg-gradient-to-r from-white via-blue-400 to-orange-400 bg-clip-text text-transparent",
                        "Ephemeral Spaces"
                    }
                    p {
                        class: "text-xl md:text-2xl text-slate-300 mb-12 max-w-2xl mx-auto leading-relaxed",
                        "Share text and files instantly with a temporary, shareable URL."
                    }

                    button {
                        class: "bg-orange-500 hover:bg-orange-600 text-slate-900 font-semibold text-lg px-8 py-6 rounded-xl shadow-lg hover:shadow-orange-500/50 hover:scale-105 transition-all duration-300 mb-20",
                        onclick: move |_| { coroutine.send(()); },
                        "Create New Space"
                    }

                    // Features grid
                    div { class: "grid md:grid-cols-3 gap-8 max-w-5xl mx-auto",

                        // Feature 1: Instant Sharing
                        div { class: "bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6 hover:bg-slate-700/50 transition-all duration-300",
                            div { class: "w-12 h-12 bg-blue-500/20 rounded-lg flex items-center justify-center mb-4 mx-auto",
                                svg {
                                    class: "w-6 h-6 text-blue-400",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M13 10V3L4 14h7v7l9-11h-7z"
                                    }
                                }
                            }
                            h3 { class: "text-lg font-semibold text-white mb-2", "Instant Sharing" }
                            p { class: "text-slate-300", "Share content with a single click. No registration required." }
                        }

                        // Feature 2: Temporary & Secure
                        div { class: "bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6 hover:bg-slate-700/50 transition-all duration-300",
                            div { class: "w-12 h-12 bg-orange-500/20 rounded-lg flex items-center justify-center mb-4 mx-auto",
                                svg {
                                    class: "w-6 h-6 text-orange-400",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
                                    }
                                }
                            }
                            h3 { class: "text-lg font-semibold text-white mb-2", "Temporary & Secure" }
                            p { class: "text-slate-300", "Content expires automatically. No permanent storage." }
                        }

                        // Feature 3: Text & Files
                        div { class: "bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6 hover:bg-slate-700/50 transition-all duration-300",
                            div { class: "w-12 h-12 bg-blue-500/20 rounded-lg flex items-center justify-center mb-4 mx-auto",
                                svg {
                                    class: "w-6 h-6 text-blue-400",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                                    }
                                }
                            }
                            h3 { class: "text-lg font-semibold text-white mb-2", "Text & Files" }
                            p { class: "text-slate-300", "Support for both text content and file uploads." }
                        }
                    }
                }
            }

            div { class: "absolute inset-0 opacity-10 pointer-events-none",
                div { class: "absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-96 h-96 rounded-full border border-blue-400/30" }
                div { class: "absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-80 h-80 rounded-full border border-orange-400/20" }
                div { class: "absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-64 h-64 rounded-full border border-blue-400/20" }
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
pub fn Space(props: SpaceProps) -> Element {
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

    let resource_state = space_resource.read();

    let floating_shapes = (0..18).map(|i| {
        let (size, shape) = match i % 5 {
            0 => ("w-6 h-6", "rounded-lg"),
            1 => ("w-8 h-8", "rounded-full"),
            2 => ("w-4 h-4", "rounded-sm"),
            3 => ("w-10 h-10", "rounded-lg"),
            _ => ("w-5 h-5", "rounded-full"),
        };

        let color = match i % 4 {
            0 => "bg-blue-500/30",
            1 => "bg-orange-400/25",
            2 => "bg-cyan-400/20",
            _ => "bg-white/15",
        };

        let delay = format!("animation-delay: {}ms;", i * 250);
        let top = format!("top: {}%;", (7 * i + 15) % 85);
        let left = format!("left: {}%;", (11 * i + 8) % 92);

        rsx! {
            div {
                class: "absolute opacity-70 animate-bounce {size} {color} {shape}",
                style: "{top} {left} {delay}"
            }
        }
    });

    // Network connection lines for Space page
    let network_lines = rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            class: "absolute inset-0 w-full h-full opacity-25 pointer-events-none",
            defs {
                linearGradient {
                    id: "spaceNetworkGradient",
                    x1: "0%", y1: "0%", x2: "100%", y2: "100%",
                    stop { offset: "0%", stop_color: "#3b82f6", stop_opacity: "0.5" }
                    stop { offset: "50%", stop_color: "#06b6d4", stop_opacity: "0.3" }
                    stop { offset: "100%", stop_color: "#f97316", stop_opacity: "0.4" }
                }
                linearGradient {
                    id: "dataFlow",
                    x1: "0%", y1: "0%", x2: "100%", y2: "0%",
                    stop { offset: "0%", stop_color: "#3b82f6", stop_opacity: "0" }
                    stop { offset: "50%", stop_color: "#06b6d4", stop_opacity: "0.8" }
                    stop { offset: "100%", stop_color: "#3b82f6", stop_opacity: "0" }
                }
            }

            // Main connection lines
            line { x1: "5%", y1: "15%", x2: "25%", y2: "35%", stroke: "url(#spaceNetworkGradient)", stroke_width: "1.5" }
            line { x1: "75%", y1: "20%", x2: "95%", y2: "40%", stroke: "url(#spaceNetworkGradient)", stroke_width: "1.5" }
            line { x1: "15%", y1: "80%", x2: "35%", y2: "90%", stroke: "url(#spaceNetworkGradient)", stroke_width: "1.5" }
            line { x1: "65%", y1: "70%", x2: "85%", y2: "85%", stroke: "url(#spaceNetworkGradient)", stroke_width: "1.5" }

            // Data flow lines
            line { x1: "20%", y1: "50%", x2: "80%", y2: "50%", stroke: "url(#dataFlow)", stroke_width: "2" }
            line { x1: "50%", y1: "20%", x2: "50%", y2: "80%", stroke: "url(#dataFlow)", stroke_width: "2" }

            // Network nodes
            circle { cx: "20%", cy: "30%", r: "2", fill: "#3b82f6", opacity: "0.6" }
            circle { cx: "80%", cy: "25%", r: "3", fill: "#06b6d4", opacity: "0.5" }
            circle { cx: "25%", cy: "75%", r: "2.5", fill: "#f97316", opacity: "0.4" }
            circle { cx: "75%", cy: "80%", r: "2", fill: "#3b82f6", opacity: "0.6" }
        }
    };

    rsx! {
        div {
            class: "min-h-screen bg-gradient-to-br from-slate-900 via-slate-800 to-blue-900 relative overflow-hidden",

            {network_lines}
            {floating_shapes}

            // Main content container
            div { class: "relative z-10 flex flex-col items-center justify-start min-h-screen px-4 py-8",

                // Header with space ID and navigation
                div { class: "w-full max-w-4xl mb-8",
                    div { class: "bg-slate-800/40 backdrop-blur-sm border border-slate-700/50 rounded-xl p-6 text-center",
                        h1 {
                            class: "text-3xl md:text-4xl font-bold mb-4 bg-gradient-to-r from-white via-blue-400 to-cyan-400 bg-clip-text text-transparent",
                            "Space: {props.id}"
                        }
                        Link {
                            to: Route::Home {},
                            class: "inline-flex items-center gap-2 text-blue-400 hover:text-blue-300 transition-colors duration-300 text-lg hover:scale-105 transform",
                            svg {
                                class: "w-5 h-5",
                                fill: "none",
                                stroke: "currentColor",
                                view_box: "0 0 24 24",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M10 19l-7-7m0 0l7-7m-7 7h18"
                                }
                            }
                            "Back to Home"
                        }
                    }
                }

                // Main workspace container
                div { class: "w-full max-w-4xl",
                    if let Some(inner) = &*resource_state {
                        match inner {
                            Some(data) => rsx! {
                                div { class: "grid gap-8 md:grid-cols-1 lg:grid-cols-2",
                                    TextBin { data: data.clone(), space_id: props.id.clone() }
                                    FileDrop {
                                        space_id: props.id.clone(),
                                        files: data.files.clone(),
                                        space_resource: space_resource.clone()
                                    }
                                }
                            },
                            None => rsx! {
                                div { class: "bg-red-900/20 backdrop-blur-sm border border-red-500/30 rounded-xl p-8 text-center",
                                    svg {
                                        class: "w-16 h-16 text-red-400 mx-auto mb-4",
                                        fill: "none",
                                        stroke: "currentColor",
                                        view_box: "0 0 24 24",
                                        path {
                                            stroke_linecap: "round",
                                            stroke_linejoin: "round",
                                            stroke_width: "2",
                                            d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"
                                        }
                                    }
                                    p { class: "text-red-300 text-xl", "Failed to load space data" }
                                }
                            }
                        }
                    } else {
                        div { class: "bg-slate-800/40 backdrop-blur-sm border border-slate-700/50 rounded-xl p-12 text-center",
                            div { class: "animate-spin w-12 h-12 border-4 border-blue-500/30 border-t-blue-500 rounded-full mx-auto mb-4" }
                            p { class: "text-slate-300 text-xl animate-pulse", "Loading space..." }
                        }
                    }
                }
            }

            // Background geometric patterns
            div { class: "absolute inset-0 opacity-5 pointer-events-none",
                div { class: "absolute top-1/4 left-1/4 w-64 h-64 border border-blue-400/20 rounded-full" }
                div { class: "absolute top-3/4 right-1/4 w-48 h-48 border border-cyan-400/15 rounded-lg rotate-45" }
                div { class: "absolute bottom-1/4 left-1/2 w-32 h-32 border border-orange-400/10 rounded-full" }
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
        div { class: "bg-slate-800/40 backdrop-blur-sm border border-slate-700/50 rounded-xl p-6 hover:bg-slate-700/30 transition-all duration-300",
            div { class: "flex items-center gap-3 mb-4",
                div { class: "w-10 h-10 bg-blue-500/20 rounded-lg flex items-center justify-center",
                    svg {
                        class: "w-5 h-5 text-blue-400",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                        }
                    }
                }
                h2 { class: "text-xl font-semibold text-white", "Text Content" }
            }

            textarea {
                class: "w-full min-h-[200px] bg-slate-900/50 border border-slate-600/50 rounded-lg p-4 text-slate-100 placeholder-slate-400 focus:border-blue-500/50 focus:ring-2 focus:ring-blue-500/20 focus:outline-none transition-all duration-300 font-mono text-sm resize-none",
                placeholder: "Enter your text content here...",
                value: "{text_content}",
                oninput: move |event| text_content.set(event.value()),
            }

            button {
                class: "mt-4 bg-gradient-to-r from-blue-600 to-blue-700 hover:from-blue-700 hover:to-blue-800 text-white font-semibold px-6 py-3 rounded-lg shadow-lg hover:shadow-blue-500/25 hover:scale-105 transition-all duration-300 flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed",
                onclick: move |_| {
                    save_coroutine.send(text_content.read().clone());
                    save_button_state.set("Saving...".to_string());

                    let mut save_button_state = save_button_state.clone();
                    spawn(async move {
                        sleep(Duration::from_secs(2)).await;
                        save_button_state.set("Save".to_string());
                    });
                },
                if save_button_state.read().as_str() == "Saving..." {
                    div { class: "animate-spin w-4 h-4 border-2 border-white/30 border-t-white rounded-full" }
                } else {
                    svg {
                        class: "w-4 h-4",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M5 13l4 4L19 7"
                        }
                    }
                },


                "{save_button_state}"
            }
        }
    }
}

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
    let upload_coroutine: Coroutine<Vec<(String, Vec<u8>)>> =
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
            p {
                class: "text-white py-4",
                "Upload files to share them temporarily."
            }

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

            p {
                class: "text-white pt-20",
                "Files uploaded:"
            }
            ul {
                class: "list-disc pl-5 mt-4 text-white",
                for file in props.files.iter() {
                    li {
                        class: "mb-2",
                        "{file.filename} ({file.size} bytes)"
                    }
                }
            }
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    launch(App);
}
