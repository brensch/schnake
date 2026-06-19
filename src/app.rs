use crate::dom::{by_id, js_err};
use crate::protocol::{GameState, COLORS};
use crate::ui::{bind_keys, bind_ui, start_loop};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    CanvasRenderingContext2d, Document, HtmlCanvasElement, RtcDataChannel, RtcPeerConnection,
    Window,
};

pub(crate) const TICK_MS: i32 = 110;

pub(crate) struct App {
    pub(crate) document: Document,
    pub(crate) window: Window,
    pub(crate) canvas: HtmlCanvasElement,
    pub(crate) ctx: CanvasRenderingContext2d,
    pub(crate) state: GameState,
    pub(crate) local_id: String,
    pub(crate) local_name: String,
    pub(crate) local_color: String,
    pub(crate) role: Role,
    pub(crate) peer_connections: Vec<RtcPeerConnection>,
    pub(crate) peer_channels: Vec<RtcDataChannel>,
    pub(crate) host_pending_connection: Option<RtcPeerConnection>,
    pub(crate) join_connection: Option<RtcPeerConnection>,
    pub(crate) join_channel: Option<RtcDataChannel>,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Role {
    Idle,
    Host,
    Client,
}

pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let window = web_sys::window().ok_or_else(|| js_err("missing window"))?;
    let document = window
        .document()
        .ok_or_else(|| js_err("missing document"))?;
    let canvas = by_id::<HtmlCanvasElement>(&document, "game")?;
    let ctx = canvas
        .get_context("2d")?
        .ok_or_else(|| js_err("missing canvas context"))?
        .dyn_into::<CanvasRenderingContext2d>()?;

    let app = Rc::new(RefCell::new(App {
        document,
        window,
        canvas,
        ctx,
        state: GameState::new(),
        local_id: format!("p{}", (js_sys::Math::random() * 1_000_000_000.0) as u32),
        local_name: "player".to_string(),
        local_color: COLORS[0].to_string(),
        role: Role::Idle,
        peer_connections: Vec::new(),
        peer_channels: Vec::new(),
        host_pending_connection: None,
        join_connection: None,
        join_channel: None,
    }));

    bind_ui(app.clone())?;
    bind_keys(app.clone())?;
    start_loop(app.clone())?;
    app.borrow().render();

    Ok(())
}
