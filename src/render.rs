use crate::app::{App, Role};
use crate::dom::{by_id, escape_html};
use crate::protocol::{GRID_H, GRID_W};
use web_sys::HtmlElement;

// Game Boy DMG palette.
const GB_DARKEST: &str = "#0f380f";
const GB_DARK: &str = "#306230";
const GB_LIGHT: &str = "#8bac0f";
const GB_LIGHTEST: &str = "#9bbc0f";

impl App {
    pub(crate) fn show_stage(&self, active_id: &str) {
        for (id, base_class) in [
            ("start-screen", "stage"),
            ("host-screen", "stage flow-screen"),
            ("join-screen", "stage flow-screen"),
            ("game-screen", "stage game-screen"),
        ] {
            if let Ok(stage) = by_id::<HtmlElement>(&self.document, id) {
                let active = if id == active_id { " active" } else { "" };
                stage.set_class_name(&format!("{base_class}{active}"));
            }
        }
        self.sync_action_visibility();
    }

    pub(crate) fn render(&self) {
        self.sync_action_visibility();
        let width = self.canvas.width() as f64;
        let height = self.canvas.height() as f64;
        let cell = (width / GRID_W as f64).min(height / GRID_H as f64);

        let board_w = GRID_W as f64 * cell;
        let board_h = GRID_H as f64 * cell;

        // LCD screen background (lightest shade).
        self.ctx.set_fill_style_str(GB_LIGHTEST);
        self.ctx.fill_rect(0.0, 0.0, width, height);

        // Subtle checkerboard so the playfield reads as pixels.
        self.ctx.set_fill_style_str(GB_LIGHT);
        for y in 0..GRID_H {
            for x in 0..GRID_W {
                if (x + y) % 2 == 0 {
                    self.ctx
                        .fill_rect(x as f64 * cell, y as f64 * cell, cell, cell);
                }
            }
        }

        // Dot-matrix grid lines.
        self.ctx.set_stroke_style_str("rgba(15, 56, 15, 0.12)");
        self.ctx.set_line_width(1.0);
        for x in 0..=GRID_W {
            let px = x as f64 * cell;
            self.ctx.begin_path();
            self.ctx.move_to(px, 0.0);
            self.ctx.line_to(px, board_h);
            self.ctx.stroke();
        }
        for y in 0..=GRID_H {
            let py = y as f64 * cell;
            self.ctx.begin_path();
            self.ctx.move_to(0.0, py);
            self.ctx.line_to(board_w, py);
            self.ctx.stroke();
        }

        // Apple: dark pixel square with a small highlight bite.
        let ax = self.state.apple.x as f64 * cell;
        let ay = self.state.apple.y as f64 * cell;
        let a_inset = (cell * 0.18).max(2.0);
        self.ctx.set_fill_style_str(GB_DARKEST);
        self.ctx
            .fill_rect(ax + a_inset, ay + a_inset, cell - a_inset * 2.0, cell - a_inset * 2.0);
        self.ctx.set_fill_style_str(GB_DARK);
        let dot = (cell * 0.16).max(2.0);
        self.ctx
            .fill_rect(ax + cell * 0.34, ay + cell * 0.28, dot, dot);

        // Snakes: solid pixel squares, each with a dark LCD outline.
        for snake in &self.state.snakes {
            for (index, point) in snake.body.iter().enumerate() {
                let head = index == 0;
                let px = point.x as f64 * cell;
                let py = point.y as f64 * cell;

                // Dark outline cell (segments touch to form a continuous border).
                self.ctx.set_fill_style_str(GB_DARKEST);
                self.ctx.fill_rect(px, py, cell, cell);

                // Inner colour block; head sits slightly larger/brighter.
                let inset = if head { cell * 0.10 } else { cell * 0.16 };
                self.ctx.set_fill_style_str(&snake.color);
                self.ctx
                    .fill_rect(px + inset, py + inset, cell - inset * 2.0, cell - inset * 2.0);

                // Eye pixel on the head for a touch of character.
                if head {
                    self.ctx.set_fill_style_str(GB_DARKEST);
                    let eye = (cell * 0.16).max(2.0);
                    self.ctx
                        .fill_rect(px + cell * 0.5 - eye / 2.0, py + cell * 0.3, eye, eye);
                }
            }
        }
    }

    pub(crate) fn render_scoreboard(&self) {
        let Ok(scoreboard) = by_id::<HtmlElement>(&self.document, "scoreboard") else {
            return;
        };
        let html = self
            .state
            .snakes
            .iter()
            .map(|snake| {
                format!(
                    r#"<span class="score"><span class="swatch" style="background:{}"></span>{}: {}</span>"#,
                    snake.color,
                    escape_html(&snake.name),
                    snake.score
                )
            })
            .collect::<Vec<_>>()
            .join("");
        scoreboard.set_inner_html(&html);
    }

    pub(crate) fn set_status(&self, message: &str) {
        if let Ok(status) = by_id::<HtmlElement>(&self.document, "status") {
            status.set_text_content(Some(message));
        }
    }

    fn sync_action_visibility(&self) {
        self.set_hidden("add-player-btn", self.role != Role::Host);
        self.set_hidden(
            "start-game-btn",
            self.role != Role::Host || self.state.started,
        );
    }

    fn set_hidden(&self, id: &str, hidden: bool) {
        let Ok(element) = by_id::<HtmlElement>(&self.document, id) else {
            return;
        };
        let class_name = element.class_name();
        let has_hidden = class_name.split_whitespace().any(|class| class == "hidden");
        if hidden && !has_hidden {
            element.set_class_name(&format!("{class_name} hidden"));
        } else if !hidden && has_hidden {
            let classes = class_name
                .split_whitespace()
                .filter(|class| *class != "hidden")
                .collect::<Vec<_>>()
                .join(" ");
            element.set_class_name(&classes);
        }
    }
}
