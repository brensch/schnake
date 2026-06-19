use crate::dom::js_err;
use crate::protocol::Signal;
use miniz_oxide::{deflate::compress_to_vec_zlib, inflate::decompress_to_vec_zlib};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    RtcConfiguration, RtcIceGatheringState, RtcIceServer, RtcPeerConnection, RtcSdpType,
    RtcSessionDescriptionInit,
};

const SIGNAL_PREFIX: &[u8] = b"SNK1";

pub fn create_peer_connection() -> Result<RtcPeerConnection, JsValue> {
    let ice_server = RtcIceServer::new();
    ice_server.set_urls(&JsValue::from_str("stun:stun.l.google.com:19302"));

    let servers = js_sys::Array::new();
    servers.push(&ice_server);

    let config = RtcConfiguration::new();
    config.set_ice_servers(&servers);

    RtcPeerConnection::new_with_configuration(&config)
}

pub async fn wait_for_ice(pc: &RtcPeerConnection) -> Result<(), JsValue> {
    let started = js_sys::Date::now();
    loop {
        let elapsed = js_sys::Date::now() - started;
        if pc.ice_gathering_state() == RtcIceGatheringState::Complete {
            return Ok(());
        }
        if local_sdp_has_candidate(pc) && elapsed >= 750.0 {
            return Ok(());
        }
        if elapsed >= 3_000.0 {
            return Ok(());
        }
        delay(100).await?;
    }
}

pub fn encode_local_description(pc: &RtcPeerConnection) -> Result<Vec<u8>, JsValue> {
    let description = pc
        .local_description()
        .ok_or_else(|| js_err("missing local description"))?;
    let signal = Signal {
        sdp_type: sdp_type_to_string(description.type_()),
        sdp: description.sdp(),
    };
    let json = serde_json::to_vec(&signal).unwrap();
    let compressed = compress_to_vec_zlib(&json, 9);
    let mut payload = Vec::with_capacity(SIGNAL_PREFIX.len() + compressed.len());
    payload.extend_from_slice(SIGNAL_PREFIX);
    payload.extend_from_slice(&compressed);
    Ok(payload)
}

pub fn decode_signal(payload: &[u8]) -> Result<Signal, JsValue> {
    if let Some(compressed) = payload.strip_prefix(SIGNAL_PREFIX) {
        let json = decompress_to_vec_zlib(compressed)
            .map_err(|_| js_err("compressed signal is invalid"))?;
        return serde_json::from_slice::<Signal>(&json)
            .map_err(|error| js_err(&format!("signal JSON is invalid: {error}")));
    }

    let text = std::str::from_utf8(payload).map_err(|_| js_err("signal is not UTF-8 JSON"))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(js_err("signal text is empty"));
    }

    if let Ok(signal) = serde_json::from_str::<Signal>(trimmed) {
        return Ok(signal);
    }

    let decoded = js_sys::decode_uri_component(trimmed)
        .map_err(|_| js_err("signal is not valid JSON or encoded JSON"))?;
    let decoded = String::from(decoded);
    serde_json::from_str::<Signal>(&decoded)
        .map_err(|error| js_err(&format!("signal JSON is invalid: {error}")))
}

pub fn session_description(signal: &Signal) -> Result<RtcSessionDescriptionInit, JsValue> {
    let sdp_type = match signal.sdp_type.as_str() {
        "offer" => RtcSdpType::Offer,
        "answer" => RtcSdpType::Answer,
        _ => return Err(js_err("unknown SDP type")),
    };
    let description = RtcSessionDescriptionInit::new(sdp_type);
    description.set_sdp(&signal.sdp);
    Ok(description)
}

fn local_sdp_has_candidate(pc: &RtcPeerConnection) -> bool {
    pc.local_description()
        .map(|description| description.sdp().contains("a=candidate"))
        .unwrap_or(false)
}

async fn delay(ms: i32) -> Result<(), JsValue> {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let closure = Closure::once_into_js(move || {
            let _ = resolve.call0(&JsValue::NULL);
        });
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                ms,
            );
        }
    });
    JsFuture::from(promise).await?;
    Ok(())
}

fn sdp_type_to_string(sdp_type: RtcSdpType) -> String {
    match sdp_type {
        RtcSdpType::Offer => "offer",
        RtcSdpType::Answer => "answer",
        RtcSdpType::Pranswer => "pranswer",
        RtcSdpType::Rollback => "rollback",
        _ => "unknown",
    }
    .to_string()
}
