use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::Document;

pub fn by_id<T: JsCast>(document: &Document, id: &str) -> Result<T, JsValue> {
    document
        .get_element_by_id(id)
        .ok_or_else(|| js_err(&format!("missing #{id}")))?
        .dyn_into::<T>()
        .map_err(|_| js_err(&format!("#{id} has unexpected type")))
}

pub fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn js_err(message: &str) -> JsValue {
    JsValue::from_str(message)
}

pub fn err_text(error: JsValue) -> String {
    if let Some(text) = error.as_string() {
        return text;
    }

    let name = js_sys::Reflect::get(&error, &JsValue::from_str("name"))
        .ok()
        .and_then(|value| value.as_string());
    let message = js_sys::Reflect::get(&error, &JsValue::from_str("message"))
        .ok()
        .and_then(|value| value.as_string());

    match (name, message) {
        (Some(name), Some(message)) if !message.is_empty() => return format!("{name}: {message}"),
        (_, Some(message)) if !message.is_empty() => return message,
        (Some(name), _) if !name.is_empty() => return name,
        _ => {}
    }

    js_sys::JSON::stringify(&error)
        .ok()
        .and_then(|text| text.as_string())
        .unwrap_or_else(|| "unknown browser error".to_string())
}
