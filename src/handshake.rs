use crate::app::{App, Role};
use crate::dom::js_err;
use crate::protocol::NetMsg;
use crate::qr::{render_qr, scan_qr_text};
use crate::rtc::{
    create_peer_connection, decode_signal, encode_local_description, session_description,
    wait_for_ice,
};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{MessageEvent, RtcDataChannel, RtcDataChannelEvent, RtcSessionDescriptionInit};

pub(crate) async fn create_host_invite(app: Rc<RefCell<App>>) -> Result<(), JsValue> {
    {
        let mut app = app.borrow_mut();
        app.sync_name();
        app.role = Role::Host;
        app.show_stage("host-screen");
        let id = app.local_id.clone();
        let name = app.local_name.clone();
        let color = app.local_color.clone();
        app.add_snake(id, name, color);
        app.set_status("Making host QR.");
        app.render();
    }

    let pc = create_peer_connection()?;
    let channel = pc.create_data_channel("schnake");
    setup_channel(app.clone(), channel.clone(), true);

    let offer = JsFuture::from(pc.create_offer()).await?;
    let offer: RtcSessionDescriptionInit = offer.unchecked_into();
    JsFuture::from(pc.set_local_description(&offer)).await?;
    app.borrow().set_status("Gathering network info.");
    wait_for_ice(&pc).await?;

    let signal = encode_local_description(&pc)?;
    app.borrow().set_status("Rendering host QR.");
    render_qr(&app.borrow().document, "host-qr", &signal)?;

    let mut app = app.borrow_mut();
    app.host_pending_connection = Some(pc.clone());
    app.peer_connections.push(pc);
    app.peer_channels.push(channel);
    app.set_status("Show this QR.");
    Ok(())
}

pub(crate) async fn scan_host_qr(app: Rc<RefCell<App>>) -> Result<(), JsValue> {
    app.borrow().set_status("Scanning host QR.");
    let offer_text = scan_qr_text("join-scan").await?;
    create_join_reply(app, &offer_text).await
}

pub(crate) async fn scan_host_reply(app: Rc<RefCell<App>>) -> Result<(), JsValue> {
    app.borrow().set_status("Scanning reply QR.");
    let answer_text = scan_qr_text("host-scan").await?;
    apply_host_reply(app, &answer_text).await
}

async fn create_join_reply(app: Rc<RefCell<App>>, offer_text: &str) -> Result<(), JsValue> {
    {
        let mut app = app.borrow_mut();
        app.sync_name();
        app.role = Role::Client;
        app.show_stage("join-screen");
        app.set_status("Making reply QR.");
    }

    let offer = decode_signal(offer_text)?;
    let pc = create_peer_connection()?;

    {
        let app = app.clone();
        let on_data_channel = Closure::wrap(Box::new(move |event: RtcDataChannelEvent| {
            let channel = event.channel();
            setup_channel(app.clone(), channel.clone(), false);
            let mut app = app.borrow_mut();
            app.join_channel = Some(channel);
        }) as Box<dyn FnMut(_)>);
        pc.set_ondatachannel(Some(on_data_channel.as_ref().unchecked_ref()));
        on_data_channel.forget();
    }

    let remote = session_description(&offer)?;
    JsFuture::from(pc.set_remote_description(&remote)).await?;
    let answer = JsFuture::from(pc.create_answer()).await?;
    let answer: RtcSessionDescriptionInit = answer.unchecked_into();
    JsFuture::from(pc.set_local_description(&answer)).await?;
    app.borrow().set_status("Gathering network info.");
    wait_for_ice(&pc).await?;

    let signal = encode_local_description(&pc)?;
    app.borrow().set_status("Rendering reply QR.");
    render_qr(&app.borrow().document, "join-qr", &signal)?;

    let mut app = app.borrow_mut();
    app.join_connection = Some(pc);
    app.set_status("Show this QR.");
    Ok(())
}

async fn apply_host_reply(app: Rc<RefCell<App>>, answer_text: &str) -> Result<(), JsValue> {
    let answer = decode_signal(answer_text)?;
    let remote = session_description(&answer)?;
    let pc = app
        .borrow()
        .host_pending_connection
        .clone()
        .ok_or_else(|| js_err("make a host QR first"))?;
    JsFuture::from(pc.set_remote_description(&remote)).await?;
    app.borrow().set_status("Connecting.");
    Ok(())
}

fn setup_channel(app: Rc<RefCell<App>>, channel: RtcDataChannel, is_host: bool) {
    let on_open = {
        let app = app.clone();
        let channel = channel.clone();
        Closure::wrap(Box::new(move || {
            let app_ref = app.borrow();
            if is_host {
                app_ref.set_status("Peer connected. Press Start when ready.");
                app_ref.show_stage("game-screen");
                app_ref.send_channel(
                    &channel,
                    &NetMsg::State {
                        state: app_ref.state.clone(),
                    },
                );
            } else {
                app_ref.set_status("Connected. Waiting for host to start.");
                app_ref.show_stage("game-screen");
                app_ref.send_channel(
                    &channel,
                    &NetMsg::Hello {
                        id: app_ref.local_id.clone(),
                        name: app_ref.local_name.clone(),
                        color: app_ref.local_color.clone(),
                    },
                );
            }
        }) as Box<dyn FnMut()>)
    };
    channel.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    on_open.forget();

    let on_message = {
        let app = app.clone();
        let channel = channel.clone();
        Closure::wrap(Box::new(move |event: MessageEvent| {
            let Some(text) = event.data().as_string() else {
                return;
            };
            let Ok(message) = serde_json::from_str::<NetMsg>(&text) else {
                return;
            };

            let mut app = app.borrow_mut();
            match (is_host, message) {
                (true, NetMsg::Hello { id, name, color }) => {
                    app.add_snake(id, name, color);
                    app.set_status("Player joined. Press Start when ready.");
                    app.show_stage("game-screen");
                    app.send_channel(
                        &channel,
                        &NetMsg::State {
                            state: app.state.clone(),
                        },
                    );
                }
                (true, NetMsg::Input { id, dir }) => app.set_input(&id, dir),
                (false, NetMsg::State { state }) => {
                    app.state = state;
                    if app.state.started {
                        app.set_status("Game on.");
                    } else {
                        app.set_status("Waiting for host to start.");
                    }
                    app.show_stage("game-screen");
                    app.render_scoreboard();
                    app.render();
                }
                _ => {}
            }
        }) as Box<dyn FnMut(_)>)
    };
    channel.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();
}
