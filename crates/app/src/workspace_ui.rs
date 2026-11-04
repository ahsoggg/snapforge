//! User-named layouts and editable editor shortcut drafts.

use eframe::egui::{self, RichText, Ui};
use snapforge_core::workspace::{EDITOR_SHORTCUTS, default_editor_keys, valid_editor_key};

use crate::i18n::tr;
use crate::{App, theme};

fn shortcut_errors(keys: &[String]) -> Vec<Option<&'static str>> {
    keys.iter()
        .map(|key| {
            let canonical_function =
                !key.starts_with('F') || key.len() == 1 || (1..=12).any(|n| *key == format!("F{n}"));
            if !valid_editor_key(key) || !canonical_function {
                Some("ws.key_invalid")
            } else if !key.is_empty() && keys.iter().filter(|other| *other == key).count() > 1 {
                Some("ws.key_duplicate")
            } else {
                None
            }
        })
        .collect()
}

impl App {
    pub fn workspace_card(&mut self, ui: &mut Ui, changed: &mut bool) {
        let lang = self.settings.language;
        theme::card(ui).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(tr(lang, "ws.title")).strong());
            let current = self.settings.active_workspace;
            let selected_name = current
                .and_then(|i| self.settings.workspace_profiles.get(i))
                .map_or(tr(lang, "ws.manual"), |profile| profile.name.as_str());
            let mut activate = None;
            let mut manual = false;
            egui::ComboBox::from_id_salt("workspace_profile")
                .selected_text(selected_name)
                .show_ui(ui, |ui| {
                    manual = ui.selectable_label(current.is_none(), tr(lang, "ws.manual")).clicked();
                    for (i, profile) in self.settings.workspace_profiles.iter().enumerate() {
                        if ui.selectable_label(current == Some(i), &profile.name).clicked() {
                            activate = Some(i);
                        }
                    }
                });
            if let Some(i) = activate {
                match self.settings.activate_workspace(i) {
                    Ok(()) => {
                        self.workspace_name = self.settings.workspace_profiles[i].name.clone();
                        self.editor_key_text.clear();
                        self.overlay_key_text.clear();
                        ui.ctx().set_zoom_factor(self.settings.ui_scale as f32 / 100.0);
                        theme::set_compact(ui.ctx(), self.settings.compact_ui);
                        *changed = true;
                    }
                    Err(error) => self.workspace_error(error),
                }
            } else if manual && current.is_some() {
                self.settings.active_workspace = None;
                self.workspace_name.clear();
                *changed = true;
            }
            ui.horizontal(|ui| {
                ui.label(tr(lang, "ws.name"));
                ui.add(egui::TextEdit::singleline(&mut self.workspace_name).desired_width(230.0));
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button(tr(lang, "ws.create")).clicked() {
                    match self.settings.create_workspace(&self.workspace_name) {
                        Ok(()) => {
                            self.workspace_name = self.workspace_name.trim().to_owned();
                            *changed = true;
                        }
                        Err(error) => self.workspace_error(error),
                    }
                }
                let active = self.settings.active_workspace.is_some();
                if ui.add_enabled(active, egui::Button::new(tr(lang, "ws.save"))).clicked() {
                    match self.settings.save_workspace() {
                        Ok(()) => *changed = true,
                        Err(error) => self.workspace_error(error),
                    }
                }
                if ui
                    .add_enabled(active, egui::Button::new(tr(lang, "ws.rename")))
                    .clicked()
                {
                    match self.settings.rename_workspace(&self.workspace_name) {
                        Ok(()) => {
                            self.workspace_name = self.workspace_name.trim().to_owned();
                            *changed = true;
                        }
                        Err(error) => self.workspace_error(error),
                    }
                }
                ui.add_enabled_ui(active, |ui| {
                    ui.menu_button(tr(lang, "ws.delete"), |ui| {
                        ui.set_max_width(300.0);
                        ui.label(tr(lang, "ws.delete_hint"));
                        if ui.button(tr(lang, "ws.delete_confirm")).clicked() {
                            match self.settings.remove_workspace() {
                                Ok(()) => {
                                    self.workspace_name.clear();
                                    *changed = true;
                                }
                                Err(error) => self.workspace_error(error),
                            }
                            ui.close();
                        }
                    });
                });
            });
            if let Some(profile) = self
                .settings
                .active_workspace
                .and_then(|i| self.settings.workspace_profiles.get(i))
                && profile.layout != self.settings.workspace_layout()
            {
                ui.colored_label(theme::WARN, tr(lang, "ws.modified"));
            }
            ui.label(RichText::new(tr(lang, "ws.help")).weak().small());
        });
    }

    fn workspace_error(&mut self, error: &str) {
        let key = match error {
            "name" => "ws.err_name",
            "duplicate" => "ws.err_duplicate",
            "limit" => "ws.err_limit",
            _ => "ws.err_missing",
        };
        self.notify(tr(self.settings.language, key), true);
    }

    pub fn editor_keys_card(&mut self, ui: &mut Ui, changed: &mut bool) {
        let lang = self.settings.language;
        if self.editor_key_text.len() != EDITOR_SHORTCUTS.len() {
            self.editor_key_text = EDITOR_SHORTCUTS
                .iter()
                .map(|(tool, _)| self.settings.editor_key(tool).to_owned())
                .collect();
        }
        theme::card(ui).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(tr(lang, "ws.editor_tools")).strong());
            ui.label(RichText::new(tr(lang, "ws.keys_hint")).weak().small());
            egui::Grid::new("editor_shortcuts_grid")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    for (i, (tool, _)) in EDITOR_SHORTCUTS.iter().enumerate() {
                        ui.label(tr(lang, tool));
                        ui.add(egui::TextEdit::singleline(&mut self.editor_key_text[i]).desired_width(70.0));
                        ui.end_row();
                    }
                });
            let normalized: Vec<String> = self
                .editor_key_text
                .iter()
                .map(|key| key.trim().to_ascii_uppercase())
                .collect();
            let errors = shortcut_errors(&normalized);
            for (i, error) in errors.iter().enumerate() {
                if let Some(key) = error {
                    ui.colored_label(
                        theme::BAD,
                        format!("{}: {}", tr(lang, EDITOR_SHORTCUTS[i].0), tr(lang, key)),
                    );
                }
            }
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        errors.iter().all(Option::is_none),
                        egui::Button::new(tr(lang, "ws.keys_apply")),
                    )
                    .clicked()
                {
                    self.settings.editor_keys = EDITOR_SHORTCUTS
                        .iter()
                        .zip(&normalized)
                        .map(|((tool, _), key)| ((*tool).to_owned(), key.clone()))
                        .collect();
                    self.editor_key_text = normalized;
                    *changed = true;
                }
                if ui.button(tr(lang, "ws.keys_reset")).clicked() {
                    self.settings.editor_keys = default_editor_keys();
                    self.editor_key_text.clear();
                    *changed = true;
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_validation_rejects_conflicts_and_noncanonical_function_keys() {
        let values = ["", "", "Q", "Q", "F12", "F01", "F+1", "Ctrl+A"].map(str::to_owned);
        assert_eq!(
            shortcut_errors(&values),
            vec![
                None,
                None,
                Some("ws.key_duplicate"),
                Some("ws.key_duplicate"),
                None,
                Some("ws.key_invalid"),
                Some("ws.key_invalid"),
                Some("ws.key_invalid")
            ]
        );
    }
}
