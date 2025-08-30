// The prelude brings all the essential Dioxus items into scope.
use dioxus::prelude::*;
use dioxus_router::prelude::*;
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message as GlooWsMessage};
use gloo_timers::future::sleep;
use js_sys;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use serde_json;
use std::time::Duration;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{window, Navigator};
use web_sys::{Blob, Url};

fn copy_to_clipboard_web(text: String) {
    spawn_local(async move {
        if let Some(win) = window() {
            let navigator: Navigator = win.navigator();

            // Check if navigator.clipboard is defined
            let navigator_js: JsValue = navigator.into();
            if let Some(clipboard_val) =
                js_sys::Reflect::get(&navigator_js, &JsValue::from_str("clipboard")).ok()
            {
                if clipboard_val.is_undefined() {
                    web_sys::console::log_1(&"Clipboard API unavailable.".into());
                    return;
                }

                // Cast JsValue -> Clipboard safely
                let clipboard: web_sys::Clipboard = clipboard_val.unchecked_into();
                let promise = clipboard.write_text(&text);
                let fut = JsFuture::from(promise);

                match fut.await {
                    Ok(_) => web_sys::console::log_1(&"Copied to clipboard!".into()),
                    Err(err) => web_sys::console::log_1(
                        &format!("Clipboard write failed: {:?}", err).into(),
                    ),
                }
            } else {
                web_sys::console::log_1(&"Clipboard property missing.".into());
            }
        } else {
            web_sys::console::log_1(&"No window available.".into());
        }
    });
}

#[derive(Routable, Clone, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[route("/")]
    Home {},
    #[route("/s/:id")]
    Hub { id: String },
}

/// The main application component that sets up the router.
#[allow(non_snake_case)]
fn App() -> Element {
    rsx! {
        main {
            class: "",
            Router::<Route> {}
        }
    }
}

#[allow(non_snake_case)]
fn Home() -> Element {
    static LOGO: Asset = asset!("/assets/logo.png");

    let navigator = use_navigator();
    // This is the correct way to handle async operations that trigger UI updates.
    let coroutine = use_coroutine(move |mut rx: UnboundedReceiver<()>| {
        // The coroutine needs its own clone of the navigator.
        let navigator = navigator.clone();
        async move {
            // Wait for a message from the `onclick` handler.
            while rx.next().await.is_some() {
                let api_url = "https://api.ephemeral-hub.com/api/hubs";

                #[derive(Deserialize, Debug)]
                struct CreateHubResponse {
                    id: String,
                }

                let client = reqwest::Client::new();
                let response = client.post(api_url).send().await;

                match response {
                    Ok(resp) => {
                        if let Ok(data) = resp.json::<CreateHubResponse>().await {
                            // Because this is run in a coroutine, the navigator
                            // update will be correctly processed by the scheduler.
                            navigator.push(Route::Hub { id: data.id });
                        } else {
                            log::error!("Failed to deserialize response from server.");
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to create hub: {}", e);
                    }
                }
            }
        }
    });

    let floating_shapes = (0..15).map(|i| {
        let (size, shape) = match i % 4 {
            0 => ("w-4 h-4", "rounded-lg"),
            1 => ("w-10 h-10", "rounded-full"),
            2 => ("w-6 h-6", "rounded-sm"),
            _ => ("w-8 h-8", "rounded-lg"),
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

            div { class: "relative z-10 flex min-h-screen",
                div { class: "text-center max-w-auto mx-auto",

                    // Logo and Main Title
                    div { class: "flex flex-col items-center justify-center mb-4",
                        img {
                            src: LOGO,
                            class: "h-32 w-auto mb-4",
                            alt: "Ephemeral Hub Logo"
                        }
                        h1 {
                            class: "text-6xl md:text-8xl font-bold bg-gradient-to-r from-white via-blue-400 to-orange-400 bg-clip-text text-transparent",
                            "Ephemeral Hub"
                        }
                    }

                    // Subtitle
                    p {
                        class: "text-xl md:text-2xl text-slate-300 mb-16 max-w-2xl mx-auto leading-relaxed",
                        "Share text + files instantly with a temporary URL"
                    }

                    // CTA Button
                    button {
                        class: "bg-orange-500 hover:bg-orange-600 text-slate-900 font-semibold text-lg px-8 py-6 rounded-xl shadow-lg hover:shadow-orange-500/50 hover:scale-105 transition-all duration-300 mb-20",
                        onclick: move |_| { coroutine.send(()); },
                        "Create New Hub"
                    }

                    // Features grid
                    div { class: "grid md:grid-cols-3 gap-8 max-w-5xl mx-auto",

                        // Feature 1: Instant Sharing
                        div { class: "bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6 hover:bg-slate-700/50 transition-all duration-300",
                            div { class: "w-12 h-12 bg-blue-500/20 rounded-lg flex items-center justify-center mb-4 mx-auto",
                                svg { class: "w-6 h-6 text-blue-400", fill: "none", stroke: "currentColor", view_box: "0 0 24 24", path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M13 10V3L4 14h7v7l9-11h-7z" } }
                            }
                            h3 { class: "text-lg font-semibold text-white mb-2", "Instant Sharing" }
                            p { class: "text-slate-300", "Share content with a single click. No registration required." }
                        }

                        // Feature 2: Temporary & Secure
                        div { class: "bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6 hover:bg-slate-700/50 transition-all duration-300",
                            div { class: "w-12 h-12 bg-orange-500/20 rounded-lg flex items-center justify-center mb-4 mx-auto",
                                svg { class: "w-6 h-6 text-orange-400", fill: "none", stroke: "currentColor", view_box: "0 0 24 24", path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" } }
                            }
                            h3 { class: "text-lg font-semibold text-white mb-2", "Temporary & Secure" }
                            p { class: "text-slate-300", "Content expires automatically. No permanent storage." }
                        }


                        // Feature 3: Text & Files
                        div { class: "bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6 hover:bg-slate-700/50 transition-all duration-300",
                            div { class: "w-12 h-12 bg-blue-500/20 rounded-lg flex items-center justify-center mb-4 mx-auto",
                                svg { class: "w-6 h-6 text-blue-400", fill: "none", stroke: "currentColor", view_box: "0 0 24 24", path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" } }
                            }
                            h3 { class: "text-lg font-semibold text-white mb-2", "Text & Files" }
                            p { class: "text-slate-300", "Support for both text content and file uploads." }
                        }
                    }

                    CliSection {}

                    div { class: "flex flex-col items-center justify-center m-10 border-t border-slate-700/50 pt-4",
                        footer {
                            class: "w-full text-center",
                            p {
                                class: "text-xs text-gray-500",
                                "Built with ❤️ by ",
                                a {
                                    class: "text-green-400 hover:text-green-300",
                                    href: "https://github.com/seahorse-byte",
                                    target: "_blank",
                                    "Olsi Gjeci"
                                }
                            }
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

#[derive(PartialEq, Clone, Copy)]
enum CliTab {
    Create,
    Pipe,
    Upload,
    Get,
}

#[allow(non_snake_case)]
fn CliSection() -> Element {
    let mut active_tab = use_signal(|| CliTab::Create);
    let copy_feedback = use_signal(String::new);

    let tab_button_class = "px-4 py-2 text-sm font-semibold rounded-md transition-colors duration-200 focus:outline-none";
    let active_class = "bg-blue-600 text-white";
    let inactive_class = "text-slate-300 hover:bg-slate-700/50";

    let tab_class = |tab: CliTab| {
        format!(
            "{} {}",
            tab_button_class,
            if active_tab() == tab {
                active_class
            } else {
                inactive_class
            }
        )
    };

    rsx! {
        div { class: "w-full max-w-4xl mx-auto mt-16",
            h2 { class: "text-3xl font-bold text-center text-white mb-8", "For Power Users" }
            div { class: "bg-slate-900/70 backdrop-blur-sm border border-slate-700 rounded-xl shadow-2xl",
                // Fake "terminal header"
                div { class: "flex items-center p-3 border-b border-slate-700",
                    div { class: "flex items-center gap-2",
                        div { class: "w-3 h-3 bg-red-500 rounded-full" }
                        div { class: "w-3 h-3 bg-yellow-500 rounded-full" }
                        div { class: "w-3 h-3 bg-green-500 rounded-full" }
                    }
                    p { class: "text-sm text-slate-400 text-center flex-grow", "ephemeral --help" }
                }

                // Tab buttons
                div { class: "p-4 flex items-center justify-center gap-2 bg-slate-800/50",
                    button {
                        class: "{tab_class(CliTab::Create)}",
                        onclick: move |_| active_tab.set(CliTab::Create),
                        "create"
                    }
                    button {
                        class: "{tab_class(CliTab::Pipe)}",
                        onclick: move |_| active_tab.set(CliTab::Pipe),
                        "pipe"
                    }
                    button {
                        class: "{tab_class(CliTab::Upload)}",
                        onclick: move |_| active_tab.set(CliTab::Upload),
                        "upload"
                    }
                    button {
                        class: "{tab_class(CliTab::Get)}",
                        onclick: move |_| active_tab.set(CliTab::Get),
                        "get"
                    }
                }

                // Active tab content
                div { class: "p-6 font-mono text-left text-sm",
                    {
                        let (comment, command) = match active_tab() {
                            CliTab::Create => ("// Instantly generates a new ephemeral hub.", "ephemeral create"),
                            CliTab::Pipe => ("// Pipes text from stdin to a hub's text bin.", "cat log.txt | ephemeral pipe <url>"),
                            CliTab::Upload => ("// Uploads a local file to a hub.", "ephemeral upload ./archive.zip <url>"),
                            CliTab::Get => ("// Downloads all content from a hub as a .zip file.", "ephemeral get <url>"),
                        };

                        let copy_button_classes = if copy_feedback().is_empty() {
                            "bg-slate-600 hover:bg-slate-500 text-white"
                        } else {
                            "bg-green-500 text-white"
                        };

                        rsx! {
                            div { class: "flex justify-between items-center bg-slate-800/50 p-4 rounded-lg",
                                div {
                                    p { class: "text-slate-300 mb-2", "{comment}" }
                                    code { class: "text-cyan-400", "$ {command}" }
                                }
                                button {
                                    class: "px-3 py-1 text-xs font-semibold rounded-md transition-colors duration-200 {copy_button_classes}",
                                   onclick: move |_| {
                                        let command = command.to_string();
                                        let mut copy_feedback = copy_feedback.clone();

                                        copy_to_clipboard_web(command.clone());

                                        spawn_local(async move {
                                            copy_feedback.set("Copied!".to_string());
                                            gloo_timers::future::sleep(std::time::Duration::from_secs(2)).await;
                                            copy_feedback.set(String::new());
                                        });
                                    },
                                    if copy_feedback().is_empty() {
                                        "Copy"
                                    } else {
                                        "Copied!"
                                    }
                                }
                            }
                        }
                    }
                }

                // Footer links
                div { class: "p-4 border-t border-slate-700 text-center flex items-center justify-center gap-6",
                    a {
                        href: "https://crates.io/crates/ephemeral_hub",
                        target: "_blank",
                        class: "text-orange-400 hover:text-orange-300 transition-colors text-sm font-semibold",
                        "Install from Crates.io"
                    }
                    a {
                        href: "https://github.com/seahorse-byte/ephemeral_hub/tags",
                        target: "_blank",
                        class: "text-blue-400 hover:text-blue-300 transition-colors text-sm font-semibold",
                        "Download from GitHub"
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Props, Clone)]
pub struct HubProps {
    id: String,
}

// This struct will hold the data we fetch from the backend.
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
struct HubData {
    id: String,
    content: String,
    created_at: String,
    files: Vec<FileInfo>,
    whiteboard: Vec<PathData>,
}

#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
struct FileInfo {
    filename: String,
    size: u64,
}

#[allow(non_snake_case)]
pub fn Hub(props: HubProps) -> Element {
    let id = props.id.clone();

    let hub_resource = use_resource(move || {
        let id = id.clone();
        async move {
            let api_url = format!("https://api.ephemeral-hub.com/api/hubs/{}", id);
            reqwest::get(&api_url)
                .await
                .ok()?
                .json::<HubData>()
                .await
                .ok()
        }
    });

    let resource_state = hub_resource.read();

    // Coroutine to handle the download process
    let download_coroutine = use_coroutine({
        let hub_id = props.id.clone();
        move |mut rx: UnboundedReceiver<()>| {
            // Clone the hub_id here, outside the async move block.
            let hub_id = hub_id.clone();
            async move {
                while rx.next().await.is_some() {
                    let api_url =
                        format!("https://api.ephemeral-hub.com/api/hubs/{}/download", hub_id);
                    let client = reqwest::Client::new();
                    match client.get(&api_url).send().await {
                        Ok(response) => {
                            if let Ok(bytes) = response.bytes().await {
                                // Create a blob from the bytes
                                let blob = Blob::new_with_u8_array_sequence(&js_sys::Array::of1(
                                    &bytes.to_vec().into(),
                                ))
                                .unwrap();

                                // Create a temporary URL for the blob
                                let url = Url::create_object_url_with_blob(&blob).unwrap();

                                // Create an anchor element to trigger the download
                                let window = web_sys::window().unwrap();
                                let document = window.document().unwrap();
                                let a = document.create_element("a").unwrap();
                                a.set_attribute("href", &url).unwrap();
                                a.set_attribute(
                                    "download",
                                    &format!("ephemeral_hub_{}.zip", hub_id),
                                )
                                .unwrap();
                                a.dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
                                    .unwrap();

                                // Clean up the temporary URL
                                Url::revoke_object_url(&url).unwrap();
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to download files: {}", e);
                        }
                    }
                }
            }
        }
    });

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

    // Network connection lines for Hub page
    let network_lines = rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            class: "absolute inset-0 w-full h-full opacity-25 pointer-events-none",
            defs {
                linearGradient {
                    id: "hubNetworkGradient",
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
            line { x1: "5%", y1: "15%", x2: "25%", y2: "35%", stroke: "url(#hubNetworkGradient)", stroke_width: "1.5" }
            line { x1: "75%", y1: "20%", x2: "95%", y2: "40%", stroke: "url(#hubNetworkGradient)", stroke_width: "1.5" }
            line { x1: "15%", y1: "80%", x2: "35%", y2: "90%", stroke: "url(#hubNetworkGradient)", stroke_width: "1.5" }
            line { x1: "65%", y1: "70%", x2: "85%", y2: "85%", stroke: "url(#hubNetworkGradient)", stroke_width: "1.5" }

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

                div {
                    class: "flex flex-row max-w-4xl",
                    // Header with hub ID and navigation
                    div { class: "w-full max-w-4xl mb-8 flex flex-col md:flex-row",
                    div { class: "bg-slate-800/40 backdrop-blur-sm border border-slate-700/50 rounded-xl p-6 text-center",
                        h1 {
                            class: "text-3xl md:text-4xl font-bold mb-4 bg-gradient-to-r from-white via-blue-400 to-cyan-400 bg-clip-text text-transparent",
                            "Hub: {props.id}"
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
                }
                div { class: "w-full max-w-4xl",
                    if let Some(inner) = &*resource_state {
                        match inner {
                            Some(data) => rsx! {
                                div { class: "grid gap-8 md:grid-cols-1 lg:grid-cols-2",
                                    TextBin { data: data.clone(), hub_id: props.id.clone() }
                                    FileDrop {
                                        hub_id: props.id.clone(),
                                        files: data.files.clone(),
                                        hub_resource: hub_resource.clone()
                                    }
                                    Whiteboard { hub_id: props.id.clone(), initial_paths: data.whiteboard.clone() }
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
                                    p { class: "text-red-300 text-xl", "Failed to load hub data" }
                                }
                            }
                        }
                    } else {
                        div { class: "bg-slate-800/40 backdrop-blur-sm border border-slate-700/50 rounded-xl p-12 text-center",
                            div { class: "animate-spin w-12 h-12 border-4 border-blue-500/30 border-t-blue-500 rounded-full mx-auto mb-4" }
                            p { class: "text-slate-300 text-xl animate-pulse", "Loading hub..." }
                        }
                    }
                }


                // Download All Button
                div { class: "p-6 text-center w-full mx-auto mt-8",
                    button {
                        class: "text-xl inline-flex items-center gap-2 px-6 py-4 bg-indigo-600 text-white font-semibold rounded-lg shadow-sm hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 transition-all duration-150",
                        onclick: move |_| download_coroutine.send(()),
                        svg {
                            class: "w-5 h-5",
                            xmlns: "http://www.w3.org/2000/svg",
                            fill: "none",
                            view_box: "0 0 24 24",
                            stroke_width: "2",
                            stroke: "currentColor",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                d: "M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3"
                            }
                        }
                        "Download All Assets"
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
    data: HubData,
    hub_id: String,
}

#[allow(non_snake_case)]
fn TextBin(props: TextBinProps) -> Element {
    let mut text_content = use_signal(|| props.data.content.clone());
    let mut save_button_state = use_signal(|| "Save".to_string());

    let hub_id = props.hub_id.clone();

    let save_coroutine = use_coroutine(move |mut rx: UnboundedReceiver<String>| {
        let hub_id = hub_id.clone();
        async move {
            while let Some(content) = rx.next().await {
                let api_url = format!("https://api.ephemeral-hub.com/api/hubs/{}/text", hub_id);
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
                        class: "w-6 h-6 text-orange-400",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"
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
                class: "inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white font-semibold rounded-lg shadow-sm hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 transition-all duration-150",
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
    hub_id: String,
    files: Vec<FileInfo>,
    hub_resource: Resource<Option<HubData>>,
}

#[allow(non_snake_case)]
fn FileDrop(props: FileDropProps) -> Element {
    let is_uploading = use_signal(|| false);

    // The coroutine now expects a Vec containing the filename and its bytes.
    let upload_coroutine: Coroutine<Vec<(String, Vec<u8>)>> =
        use_coroutine(move |mut rx: UnboundedReceiver<Vec<(String, Vec<u8>)>>| {
            let hub_id = props.hub_id.clone();
            let mut hub_resource = props.hub_resource.clone();
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
                    let api_url =
                        format!("https://api.ephemeral-hub.com/api/hubs/{}/files", hub_id);

                    let res = client.post(api_url).multipart(form).send().await;

                    if res.is_ok() {
                        hub_resource.restart();
                    } else {
                        log::error!("Failed to upload files: {:?}", res.err());
                    }
                    is_uploading.set(false);
                }
            }
        });

    rsx! {
        div {
            class: "bg-slate-800/40 backdrop-blur-sm border border-slate-700/50 rounded-xl p-6 hover:bg-slate-700/30 transition-all duration-300",
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
                h2 { class: "text-xl font-semibold text-white", "FILES" }
            }
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
                class: "cursor-pointer inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white font-semibold rounded-lg shadow-sm hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 transition-all duration-150",
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

// Data structure for a single drawing path
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct PathData {
    id: String,
    points: Vec<(f64, f64)>,
    color: String,
    stroke_width: f64,
}

// Message format for WebSocket communication
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
enum WsMessage {
    PathCompleted(PathData),
}

#[derive(PartialEq, Props, Clone)]
struct WhiteboardProps {
    hub_id: String,
    initial_paths: Vec<PathData>,
}
#[allow(non_snake_case)]
fn Whiteboard(props: WhiteboardProps) -> Element {
    let mut paths = use_signal(|| props.initial_paths.clone());
    // A signal to track the path currently being drawn by the user
    let mut current_path = use_signal::<Option<PathData>>(|| None);
    // Generate a unique ID for this user
    let user_id = use_memo(|| Uuid::new_v4().to_string());

    // A helper: derive color from user_id (stable hash → pick from palette)
    fn color_for_user(user_id: &str) -> String {
        let palette = vec![
            "#e6194b", "#3cb44b", "#ffe119", "#4363d8", "#f58231", "#911eb4", "#46f0f0", "#f032e6",
            "#bcf60c", "#fabebe", "#008080", "#e6beff", "#9a6324", "#fffac8", "#800000", "#aaffc3",
            "#808000", "#ffd8b1", "#000075", "#808080",
        ];
        let mut hash = 0u64;
        for b in user_id.as_bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(*b as u64);
        }
        let idx = (hash % palette.len() as u64) as usize;
        palette[idx].to_string()
    }

    let my_color = color_for_user(&user_id());

    let ws_coroutine = use_coroutine(move |mut rx: UnboundedReceiver<WsMessage>| {
        let paths = paths.clone();
        let ws_url = format!("wss://api.ephemeral-hub.com/ws/hubs/{}", props.hub_id);

        async move {
            let ws = match WebSocket::open(&ws_url) {
                Ok(ws) => ws,
                Err(e) => {
                    log::error!("Failed to connect to WebSocket: {:?}", e);
                    return;
                }
            };

            let (mut write, mut read) = ws.split();

            // Incoming messages
            spawn({
                let mut paths = paths.clone();
                async move {
                    while let Some(Ok(GlooWsMessage::Text(text))) = read.next().await {
                        if let Ok(client_msg) = serde_json::from_str::<WsMessage>(&text) {
                            match client_msg {
                                WsMessage::PathCompleted(new_path) => {
                                    paths.write().push(new_path);
                                }
                            }
                        }
                    }
                }
            });

            // Outgoing messages
            while let Some(msg_to_send) = rx.next().await {
                let json_msg = serde_json::to_string(&msg_to_send).unwrap();
                if write.send(GlooWsMessage::Text(json_msg)).await.is_err() {
                    log::error!("WebSocket connection closed. Cannot send message.");
                    break;
                }
            }
        }
    });

    let to_svg_path = |points: &Vec<(f64, f64)>| -> String {
        if points.is_empty() {
            return String::new();
        }
        let mut d = format!("M {} {}", points[0].0, points[0].1);
        for p in points.iter().skip(1) {
            d.push_str(&format!(" L {} {}", p.0, p.1));
        }
        d
    };

    rsx! {
        div {
            class: "bg-slate-800/40 backdrop-blur-sm border border-slate-700/50 rounded-xl p-6 hover:bg-slate-700/30 transition-all duration-300 col-span-1 lg:col-span-2",
            h2 { class: "text-xl font-bold text-white mb-4", "Collaborative Whiteboard" }

            svg {
                class: "w-full h-[400px] border border-gray-300 rounded-md bg-gray-50",
                prevent_default: "onmousedown onmousemove",

                onmousedown: move |evt| {
                    let path_id = format!("{}-{}", user_id, Uuid::new_v4());
                    let new_path = PathData {
                        id: path_id.clone(),
                        points: vec![(evt.element_coordinates().x, evt.element_coordinates().y)],
                        color: my_color.clone(),
                        stroke_width: 2.0,
                    };
                    current_path.set(Some(new_path));
                },

                onmousemove: move |evt| {
                    if let Some(path) = current_path.write().as_mut() {
                        let point = (evt.element_coordinates().x, evt.element_coordinates().y);
                        path.points.push(point);
                    }
                },

                onmouseup: move |_| {
                    if let Some(path) = current_path.take() {
                        // Add the completed path to our local state immediately for responsiveness.
                        paths.write().push(path.clone());
                        // Send the completed path to the server.
                        ws_coroutine.send(WsMessage::PathCompleted(path));
                    }
                },

                onmouseleave: move |_| {
                    if let Some(path) = current_path.take() {
                        // Add the completed path to our local state immediately.
                        paths.write().push(path.clone());
                        // Send the completed path to the server.
                        ws_coroutine.send(WsMessage::PathCompleted(path));
                    }
                },


                // Render all paths
                for path in paths.read().iter() {
                    path {
                        d: "{to_svg_path(&path.points)}",
                        stroke: "{path.color}",
                        stroke_width: "{path.stroke_width}",
                        fill: "none",
                        stroke_linecap: "round",
                        stroke_linejoin: "round"
                    }
                }
                // Render the path currently being drawn by this user
                if let Some(path) = current_path.read().as_ref() {
                    path {
                        d: "{to_svg_path(&path.points)}",
                        stroke: "{path.color}",
                        stroke_width: "{path.stroke_width}",
                        fill: "none",
                        stroke_linecap: "round",
                        stroke_linejoin: "round"
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
