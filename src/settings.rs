use crate::app::App;
use crate::dom::by_id;
use crate::protocol::COLORS;
use web_sys::{HtmlElement, HtmlInputElement, Storage};

const NAME_KEY: &str = "schnake.name";
const COLOR_KEY: &str = "schnake.color";

impl App {
    pub(crate) fn load_profile(&mut self) {
        let Some(storage) = self.storage() else {
            self.apply_profile_to_dom();
            return;
        };

        if let Ok(Some(name)) = storage.get_item(NAME_KEY) {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                self.local_name = trimmed.chars().take(18).collect();
            }
        }

        if let Ok(Some(color)) = storage.get_item(COLOR_KEY) {
            if COLORS.contains(&color.as_str()) {
                self.local_color = color;
            }
        }

        self.apply_profile_to_dom();
    }

    pub(crate) fn sync_profile_from_dom(&mut self) {
        if let Ok(input) = by_id::<HtmlInputElement>(&self.document, "settings-name") {
            let trimmed = input.value().trim().to_string();
            if !trimmed.is_empty() {
                self.local_name = trimmed.chars().take(18).collect();
            }
        }

        self.save_profile();
        self.apply_profile_to_dom();
    }

    pub(crate) fn set_local_color(&mut self, color: &str) {
        if COLORS.contains(&color) {
            self.local_color = color.to_string();
            self.save_profile();
            self.apply_profile_to_dom();
        }
    }

    fn storage(&self) -> Option<Storage> {
        self.window.local_storage().ok().flatten()
    }

    fn save_profile(&self) {
        let Some(storage) = self.storage() else {
            return;
        };
        let _ = storage.set_item(NAME_KEY, &self.local_name);
        let _ = storage.set_item(COLOR_KEY, &self.local_color);
    }

    fn apply_profile_to_dom(&self) {
        if let Ok(input) = by_id::<HtmlInputElement>(&self.document, "settings-name") {
            input.set_value(&self.local_name);
        }

        for (index, color) in COLORS.iter().enumerate() {
            if let Ok(button) = by_id::<HtmlElement>(&self.document, &format!("color-{index}")) {
                let selected = if *color == self.local_color {
                    " selected"
                } else {
                    ""
                };
                button.set_class_name(&format!("color-choice{selected}"));
            }
        }
    }
}
