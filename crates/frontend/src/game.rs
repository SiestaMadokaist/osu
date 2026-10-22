use std::{cell::RefCell, rc::Rc};

use leptos::{html, prelude::*};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{CanvasRenderingContext2d, HtmlAudioElement, HtmlCanvasElement};

use crate::Map;

const WIDTH: f64 = 560.0;
const HEIGHT: f64 = 720.0;
const HIT_Y: f64 = 620.0;
/// Seconds a note takes to fall from the top of the canvas to the hit line.
const FALL_SECS: f64 = 1.5;
/// Seconds a note lingers (fading) after crossing the hit line.
const FADE_SECS: f64 = 0.15;
const COLORS: [&str; 7] = [
    "#ff4d6d", "#ff8e3c", "#ffd23f", "#b5e48c", "#4cc9f0", "#7b8cff", "#d67bff",
];

#[component]
pub fn Game(map: Map) -> impl IntoView {
    let canvas = NodeRef::<html::Canvas>::new();
    let started = RwSignal::new(false);
    let map = Rc::new(map);

    // Draw an idle frame as soon as the canvas exists.
    {
        let map = map.clone();
        canvas.on_load(move |el| draw(&context(&el), &map, 0.0));
    }

    let start = {
        let map = map.clone();
        move |_| {
            let Some(el) = canvas.get() else { return };
            let ctx = context(&el);
            let audio = HtmlAudioElement::new_with_src(&map.audio).unwrap();
            let _ = audio.play();
            started.set(true);

            let map = map.clone();
            let tick: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::default();
            let next = tick.clone();
            *tick.borrow_mut() = Some(Closure::new(move || {
                draw(&ctx, &map, audio.current_time());
                let window = web_sys::window().unwrap();
                let _ = window.request_animation_frame(
                    next.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
                );
            }));
            let window = web_sys::window().unwrap();
            let _ = window
                .request_animation_frame(tick.borrow().as_ref().unwrap().as_ref().unchecked_ref());
        }
    };

    view! {
        <button on:click=start disabled=move || started.get()>"Start"</button>
        <canvas node_ref=canvas width=WIDTH height=HEIGHT></canvas>
    }
}

fn context(canvas: &HtmlCanvasElement) -> CanvasRenderingContext2d {
    canvas.get_context("2d").unwrap().unwrap().dyn_into().unwrap()
}

fn draw(ctx: &CanvasRenderingContext2d, map: &Map, now: f64) {
    let lanes = map.lanes.len().max(1);
    let lane_w = WIDTH / lanes as f64;

    ctx.clear_rect(0.0, 0.0, WIDTH, HEIGHT);
    ctx.set_stroke_style_str("#2a2a3a");
    ctx.set_line_width(1.0);
    for i in 0..=lanes {
        let x = i as f64 * lane_w;
        ctx.begin_path();
        ctx.move_to(x, 0.0);
        ctx.line_to(x, HEIGHT);
        ctx.stroke();
    }
    ctx.set_stroke_style_str("#ffffff");
    ctx.set_line_width(3.0);
    ctx.begin_path();
    ctx.move_to(0.0, HIT_Y);
    ctx.line_to(WIDTH, HIT_Y);
    ctx.stroke();

    ctx.set_fill_style_str("#888");
    ctx.set_font("12px sans-serif");
    ctx.set_text_align("center");
    for (i, lane) in map.lanes.iter().enumerate() {
        let _ = ctx.fill_text(&lane.name, (i as f64 + 0.5) * lane_w, HEIGHT - 20.0);
    }

    let speed = HIT_Y / FALL_SECS;
    let first = map.notes.partition_point(|n| n.t < now - FADE_SECS);
    for note in map.notes[first..].iter().take_while(|n| n.t <= now + FALL_SECS) {
        let dt = note.t - now;
        let y = HIT_Y - dt * speed;
        let x = (note.lane as f64 + 0.5) * lane_w;
        let radius = 10.0 + note.s.min(1.0) * 12.0;
        ctx.set_global_alpha(if dt < 0.0 { 1.0 + dt / FADE_SECS } else { 1.0 });
        ctx.set_fill_style_str(COLORS[note.lane % COLORS.len()]);
        ctx.begin_path();
        let _ = ctx.arc(x, y, radius, 0.0, std::f64::consts::TAU);
        ctx.fill();
    }
    ctx.set_global_alpha(1.0);
}
