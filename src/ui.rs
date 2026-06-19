use crate::app::{App, Role, TICK_MS};
use crate::dom::{by_id, err_text};
use crate::handshake::{create_host_invite, scan_host_qr, scan_host_reply};
use crate::protocol::{Direction, NetMsg, COLORS};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Document, Event, HtmlButtonElement, HtmlElement, HtmlInputElement, KeyboardEvent};

pub(crate) fn bind_ui(app: Rc<RefCell<App>>) -> Result<(), JsValue> {
    let document = app.borrow().document.clone();

    bind_click(&document, "host-start-btn", {
        let app = app.clone();
        move || {
            let app = app.clone();
            spawn_local(async move {
                if let Err(error) = create_host_invite(app.clone()).await {
                    app.borrow()
                        .set_status(&format!("Host failed: {}", err_text(error)));
                }
            });
        }
    })?;

    bind_click(&document, "join-start-btn", {
        let app = app.clone();
        move || {
            let mut app = app.borrow_mut();
            app.sync_name();
            app.role = Role::Client;
            app.show_stage("join-screen");
            app.set_status("Scan the host QR.");
            app.render();
        }
    })?;

    bind_click(&document, "join-scan-host-btn", {
        let app = app.clone();
        move || {
            let app = app.clone();
            spawn_local(async move {
                if let Err(error) = scan_host_qr(app.clone()).await {
                    app.borrow()
                        .set_status(&format!("Scan failed: {}", err_text(error)));
                }
            });
        }
    })?;

    bind_click(&document, "host-scan-reply-btn", {
        let app = app.clone();
        move || {
            let app = app.clone();
            spawn_local(async move {
                if let Err(error) = scan_host_reply(app.clone()).await {
                    app.borrow()
                        .set_status(&format!("Scan failed: {}", err_text(error)));
                }
            });
        }
    })?;

    bind_click(&document, "add-player-btn", {
        let app = app.clone();
        move || {
            let app = app.clone();
            spawn_local(async move {
                if let Err(error) = create_host_invite(app.clone()).await {
                    app.borrow()
                        .set_status(&format!("Host failed: {}", err_text(error)));
                }
            });
        }
    })?;

    bind_click(&document, "start-game-btn", {
        let app = app.clone();
        move || {
            app.borrow_mut().start_game();
        }
    })?;

    bind_click(&document, "settings-btn", {
        let document = document.clone();
        move || toggle_hidden(&document, "settings-panel")
    })?;

    bind_input(&document, "settings-name", {
        let app = app.clone();
        move || {
            app.borrow_mut().sync_profile_from_dom();
        }
    })?;

    for (index, color) in COLORS.iter().enumerate() {
        bind_click(&document, &format!("color-{index}"), {
            let app = app.clone();
            let color = color.to_string();
            move || {
                app.borrow_mut().set_local_color(&color);
            }
        })?;
    }

    bind_direction_button(&document, "move-up", app.clone(), Direction::Up)?;
    bind_direction_button(&document, "move-down", app.clone(), Direction::Down)?;
    bind_direction_button(&document, "move-left", app.clone(), Direction::Left)?;
    bind_direction_button(&document, "move-right", app.clone(), Direction::Right)?;

    Ok(())
}

pub(crate) fn bind_keys(app: Rc<RefCell<App>>) -> Result<(), JsValue> {
    let document = app.borrow().document.clone();
    let closure = Closure::wrap(Box::new(move |event: KeyboardEvent| {
        let dir = match event.key().as_str() {
            "ArrowUp" | "w" | "W" => Some(Direction::Up),
            "ArrowDown" | "s" | "S" => Some(Direction::Down),
            "ArrowLeft" | "a" | "A" => Some(Direction::Left),
            "ArrowRight" | "d" | "D" => Some(Direction::Right),
            _ => None,
        };

        if let Some(dir) = dir {
            event.prevent_default();
            apply_direction(&app, dir);
        }
    }) as Box<dyn FnMut(_)>);
    document.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(())
}

pub(crate) fn start_loop(app: Rc<RefCell<App>>) -> Result<(), JsValue> {
    let loop_app = app.clone();
    let closure = Closure::wrap(Box::new(move || {
        let mut app = loop_app.borrow_mut();
        if app.role == Role::Host && app.state.started {
            app.tick_host();
            app.broadcast_state();
        }
        app.render();
    }) as Box<dyn FnMut()>);
    app.borrow()
        .window
        .set_interval_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            TICK_MS,
        )?;
    closure.forget();
    Ok(())
}

fn apply_direction(app: &Rc<RefCell<App>>, dir: Direction) {
    let mut app = app.borrow_mut();
    match app.role {
        Role::Host => {
            let id = app.local_id.clone();
            app.set_input(&id, dir);
        }
        Role::Client => app.send_to_host(&NetMsg::Input {
            id: app.local_id.clone(),
            dir,
        }),
        Role::Idle => {}
    }
}

fn bind_direction_button(
    document: &Document,
    id: &str,
    app: Rc<RefCell<App>>,
    dir: Direction,
) -> Result<(), JsValue> {
    bind_click(document, id, move || apply_direction(&app, dir))
}

fn bind_click<F>(document: &Document, id: &str, handler: F) -> Result<(), JsValue>
where
    F: 'static + Fn(),
{
    let button = by_id::<HtmlButtonElement>(document, id)?;
    let closure = Closure::wrap(Box::new(move |_event: Event| handler()) as Box<dyn FnMut(_)>);
    button.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(())
}

fn bind_input<F>(document: &Document, id: &str, handler: F) -> Result<(), JsValue>
where
    F: 'static + Fn(),
{
    let input = by_id::<HtmlInputElement>(document, id)?;
    let closure = Closure::wrap(Box::new(move |_event: Event| handler()) as Box<dyn FnMut(_)>);
    input.add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(())
}

fn toggle_hidden(document: &Document, id: &str) {
    let Ok(element) = by_id::<HtmlElement>(document, id) else {
        return;
    };
    let class_name = element.class_name();
    let has_hidden = class_name.split_whitespace().any(|class| class == "hidden");
    if has_hidden {
        let classes = class_name
            .split_whitespace()
            .filter(|class| *class != "hidden")
            .collect::<Vec<_>>()
            .join(" ");
        element.set_class_name(&classes);
    } else {
        element.set_class_name(&format!("{class_name} hidden"));
    }
}
