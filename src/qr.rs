use crate::dom::{by_id, js_err};
use qrcodegen::{QrCode, QrCodeEcc};
use quircs::Quirc;
use rqrr::PreparedImage;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    CanvasRenderingContext2d, ContextAttributes2d, Document, HtmlCanvasElement, HtmlElement,
    HtmlVideoElement, MediaStream, MediaStreamConstraints, MediaStreamTrack, MediaTrackConstraints,
};

pub async fn scan_qr_payload(container_id: &str) -> Result<Vec<u8>, JsValue> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| js_err("missing document"))?;
    let container = by_id::<HtmlElement>(&document, container_id)?;
    container.set_inner_html("");

    let stream = open_camera().await?;
    let video = create_video(&document, &stream)?;
    container.append_child(&video)?;

    let result = scan_video_frames(&document, &video).await;
    stop_stream(&stream);
    container.set_inner_html("");
    result
}

pub fn render_qr(document: &Document, id: &str, payload: &[u8]) -> Result<(), JsValue> {
    let element = by_id::<HtmlElement>(document, id)?;
    element.set_inner_html(&qr_svg(payload)?);
    element.set_attribute("data-signal-bytes", &payload.len().to_string())?;
    Ok(())
}

async fn open_camera() -> Result<MediaStream, JsValue> {
    let window = web_sys::window().ok_or_else(|| js_err("missing window"))?;
    if !window.is_secure_context() {
        return Err(js_err("camera needs HTTPS or localhost"));
    }

    let media_devices = window
        .navigator()
        .media_devices()
        .map_err(|_| js_err("camera access is not available in this browser"))?;
    let constraints = MediaStreamConstraints::new();
    let video = MediaTrackConstraints::new();
    video.set_facing_mode(&JsValue::from_str("environment"));
    video.set_width(&JsValue::from_f64(1280.0));
    video.set_height(&JsValue::from_f64(720.0));

    constraints.set_audio_bool(false);
    constraints.set_video(&video);

    let stream = JsFuture::from(media_devices.get_user_media_with_constraints(&constraints)?)
        .await?
        .dyn_into::<MediaStream>()?;
    Ok(stream)
}

fn create_video(document: &Document, stream: &MediaStream) -> Result<HtmlVideoElement, JsValue> {
    let video = document
        .create_element("video")?
        .dyn_into::<HtmlVideoElement>()?;
    video.set_muted(true);
    video.set_autoplay(true);
    video.set_attribute("playsinline", "true")?;
    video.set_src_object(Some(stream));
    let _ = video.play()?;
    Ok(video)
}

async fn scan_video_frames(
    document: &Document,
    video: &HtmlVideoElement,
) -> Result<Vec<u8>, JsValue> {
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;
    let options = ContextAttributes2d::new();
    options.set_will_read_frequently(true);
    let ctx = canvas
        .get_context_with_context_options("2d", &options)?
        .ok_or_else(|| js_err("missing scanner canvas context"))?
        .dyn_into::<CanvasRenderingContext2d>()?;

    let started = js_sys::Date::now();
    loop {
        let width = video.video_width();
        let height = video.video_height();
        if width > 0 && height > 0 {
            let scan_width = width.min(960);
            let scan_height = ((height as f64) * (scan_width as f64 / width as f64))
                .round()
                .max(1.0) as u32;
            canvas.set_width(scan_width);
            canvas.set_height(scan_height);
            ctx.draw_image_with_html_video_element_and_dw_and_dh(
                video,
                0.0,
                0.0,
                scan_width as f64,
                scan_height as f64,
            )?;

            let image = ctx.get_image_data(0.0, 0.0, scan_width as f64, scan_height as f64)?;
            if let Some(payload) =
                decode_rgba(scan_width as usize, scan_height as usize, &image.data().0)
            {
                return Ok(payload);
            }
        }

        if js_sys::Date::now() - started > 60_000.0 {
            return Err(js_err("No QR code found"));
        }
        delay(120).await?;
    }
}

fn decode_rgba(width: usize, height: usize, rgba: &[u8]) -> Option<Vec<u8>> {
    let mut gray = Vec::with_capacity(width * height);
    for pixel in rgba.chunks_exact(4) {
        let value =
            (0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32) as u8;
        gray.push(value);
    }

    decode_with_rqrr(width, height, &gray).or_else(|| decode_with_quircs(width, height, &gray))
}

fn decode_with_rqrr(width: usize, height: usize, gray: &[u8]) -> Option<Vec<u8>> {
    let mut image =
        PreparedImage::prepare_from_greyscale(width, height, |x, y| gray[y * width + x]);
    image.detect_grids().into_iter().find_map(|grid| {
        let mut payload = Vec::new();
        grid.decode_to(&mut payload).ok().map(|_meta| payload)
    })
}

fn decode_with_quircs(width: usize, height: usize, gray: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = Quirc::default();
    for code in decoder.identify(width, height, gray).flatten() {
        if let Ok(decoded) = code.decode() {
            return Some(decoded.payload);
        }
    }
    None
}

fn stop_stream(stream: &MediaStream) {
    for track in stream.get_tracks().iter() {
        if let Ok(track) = track.dyn_into::<MediaStreamTrack>() {
            track.stop();
        }
    }
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

fn qr_svg(payload: &[u8]) -> Result<String, JsValue> {
    let qr = QrCode::encode_binary(payload, QrCodeEcc::Low)
        .map_err(|_| js_err("QR payload is too large"))?;
    let border = 4;
    let size = qr.size();
    let view_size = size + border * 2;
    let mut path = String::new();

    for y in 0..size {
        for x in 0..size {
            if qr.get_module(x, y) {
                path.push_str(&format!("M{} {}h1v1h-1z", x + border, y + border));
            }
        }
    }

    Ok(format!(
        r##"<svg viewBox="0 0 {view_size} {view_size}" role="img" aria-label="Connection QR" xmlns="http://www.w3.org/2000/svg"><rect width="100%" height="100%" fill="#fff"/><path d="{path}" fill="#111"/></svg>"##
    ))
}
