mod app;
mod dom;
mod game;
mod handshake;
mod protocol;
mod qr;
mod render;
mod rtc;
mod settings;
mod ui;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    app::start()
}
