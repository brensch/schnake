use crate::app::App;
use crate::dom::{by_id, escape_html};
use crate::protocol::{GRID_H, GRID_W};
use web_sys::HtmlElement;

impl App {
    pub(crate) fn show_stage(&self, active_id: &str) {
        for id in ["start-panel", "host-panel", "join-panel"] {
            if let Ok(stage) = by_id::<HtmlElement>(&self.document, id) {
                let panel = if id == "start-panel" { "" } else { " panel" };
                let active = if id == active_id { " active" } else { "" };
                stage.set_class_name(&format!("stage{panel}{active}"));
            }
        }
    }

    pub(crate) fn render(&self) {
        let width = self.canvas.width() as f64;
        let height = self.canvas.height() as f64;
        let cell = (width / GRID_W as f64).min(height / GRID_H as f64);

        self.ctx.set_fill_style_str("#f9fbf8");
        self.ctx.fill_rect(0.0, 0.0, width, height);

        self.ctx.set_stroke_style_str("#dfe7e1");
        self.ctx.set_line_width(1.0);
        for x in 0..=GRID_W {
            let px = x as f64 * cell;
            self.ctx.begin_path();
            self.ctx.move_to(px, 0.0);
            self.ctx.line_to(px, GRID_H as f64 * cell);
            self.ctx.stroke();
        }
        for y in 0..=GRID_H {
            let py = y as f64 * cell;
            self.ctx.begin_path();
            self.ctx.move_to(0.0, py);
            self.ctx.line_to(GRID_W as f64 * cell, py);
            self.ctx.stroke();
        }

        self.ctx.set_fill_style_str("#d83b2d");
        self.ctx.fill_rect(
            self.state.apple.x as f64 * cell + 4.0,
            self.state.apple.y as f64 * cell + 4.0,
            cell - 8.0,
            cell - 8.0,
        );

        for snake in &self.state.snakes {
            self.ctx.set_fill_style_str(&snake.color);
            for (index, point) in snake.body.iter().enumerate() {
                let inset = if index == 0 { 2.0 } else { 4.0 };
                self.ctx.fill_rect(
                    point.x as f64 * cell + inset,
                    point.y as f64 * cell + inset,
                    cell - inset * 2.0,
                    cell - inset * 2.0,
                );
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
}
