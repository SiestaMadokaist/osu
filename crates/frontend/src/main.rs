mod game;

use gloo_net::http::Request;
use leptos::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Clone, PartialEq)]
pub struct Lane {
    pub name: String,
}

#[derive(Deserialize, Clone, PartialEq)]
pub struct Note {
    pub t: f64,
    pub lane: usize,
    pub s: f64,
}

#[derive(Deserialize, Clone, PartialEq)]
pub struct Map {
    pub audio: String,
    pub lanes: Vec<Lane>,
    pub notes: Vec<Note>,
}

async fn load_map() -> Result<Map, String> {
    let res = Request::get("map.json").send().await.map_err(|e| e.to_string())?;
    res.json().await.map_err(|e| e.to_string())
}

#[component]
fn App() -> impl IntoView {
    let map = LocalResource::new(load_map);
    view! {
        <h2>"osu"</h2>
        <Suspense fallback=|| view! { <p>"loading map..."</p> }>
            {move || map.get().map(|res| match res {
                Ok(map) => view! { <game::Game map /> }.into_any(),
                Err(e) => view! { <p>"failed to load map.json: " {e}</p> }.into_any(),
            })}
        </Suspense>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
