use crate::dom::{by_id, js_err};
use qrcodegen::{QrCode, QrCodeEcc};
use wasm_bindgen::prelude::*;
use web_sys::{Document, HtmlElement};

#[wasm_bindgen(inline_js = r#"
export async function scan_qr(containerId) {
  const container = document.getElementById(containerId);
  if (!container) throw new Error("Missing scanner surface");
  if (!("BarcodeDetector" in window)) {
    throw new Error("QR scanning is not available in this browser");
  }
  if (!navigator.mediaDevices?.getUserMedia) {
    throw new Error("Camera access is not available on this page");
  }

  const supported = BarcodeDetector.getSupportedFormats
    ? await BarcodeDetector.getSupportedFormats()
    : ["qr_code"];
  if (!supported.includes("qr_code")) {
    throw new Error("This browser cannot scan QR codes");
  }

  const detector = new BarcodeDetector({ formats: ["qr_code"] });
  const stream = await navigator.mediaDevices.getUserMedia({
    video: { facingMode: { ideal: "environment" } },
    audio: false,
  });

  const video = document.createElement("video");
  video.muted = true;
  video.playsInline = true;
  video.autoplay = true;
  video.srcObject = stream;
  container.replaceChildren(video);
  await video.play();

  return await new Promise((resolve, reject) => {
    let done = false;
    const stop = () => {
      if (done) return;
      done = true;
      stream.getTracks().forEach((track) => track.stop());
      container.replaceChildren();
    };
    const timeout = window.setTimeout(() => {
      stop();
      reject(new Error("No QR code found"));
    }, 60000);

    const frame = async () => {
      if (done) return;
      try {
        const codes = await detector.detect(video);
        if (codes.length > 0 && codes[0].rawValue) {
          window.clearTimeout(timeout);
          const value = codes[0].rawValue;
          stop();
          resolve(value);
          return;
        }
      } catch (_) {
        // Some browsers throw until the first video frame is ready.
      }
      window.requestAnimationFrame(frame);
    };
    frame();
  });
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn scan_qr(container_id: &str) -> Result<JsValue, JsValue>;
}

pub async fn scan_qr_text(container_id: &str) -> Result<String, JsValue> {
    scan_qr(container_id)
        .await?
        .as_string()
        .ok_or_else(|| js_err("QR did not contain text"))
}

pub fn render_qr(document: &Document, id: &str, text: &str) -> Result<(), JsValue> {
    let element = by_id::<HtmlElement>(document, id)?;
    element.set_inner_html(&qr_svg(text)?);
    element.set_attribute("data-signal", text)?;
    Ok(())
}

fn qr_svg(text: &str) -> Result<String, JsValue> {
    let qr =
        QrCode::encode_text(text, QrCodeEcc::Low).map_err(|_| js_err("QR payload is too large"))?;
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
