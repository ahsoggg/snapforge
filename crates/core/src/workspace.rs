//! User-owned interface layouts and editor key bindings.

use crate::settings::{OverlayTool, Settings, ToolbarPos};
use serde::{Deserialize, Serialize};

pub const EDITOR_SHORTCUTS: [(&str, &str); 16] = [
    ("tool.select", "V"),
    ("tool.crop", "C"),
    ("tool.arrow", "A"),
    ("tool.line", "L"),
    ("tool.rect", "R"),
    ("tool.ellipse", "E"),
    ("tool.text", "T"),
    ("tool.pencil", "P"),
    ("tool.marker", "M"),
    ("tool.blur", "B"),
    ("tool.pixelate", "X"),
    ("tool.spotlight", "H"),
    ("tool.step", "N"),
    ("tool.chart", "G"),
    ("tool.eraser", "D"),
    ("tool.picker", "I"),
];

pub fn default_editor_keys() -> Vec<(String, String)> {
    EDITOR_SHORTCUTS
        .iter()
        .map(|(tool, key)| ((*tool).into(), (*key).into()))
        .collect()
}

pub fn valid_editor_key(key: &str) -> bool {
    key.is_empty()
        || (key.len() == 1 && key.as_bytes()[0].is_ascii_alphanumeric())
        || key.strip_prefix('F').is_some_and(|text| {
            text.parse::<u8>()
                .is_ok_and(|n| (1..=12).contains(&n) && n.to_string() == text)
        })
}

pub fn sanitize_editor_keys(keys: &mut Vec<(String, String)>) {
    let mut clean: Vec<(String, String)> = Vec::new();
    for (tool, key) in keys.iter() {
        let Some((_, fallback)) = EDITOR_SHORTCUTS.iter().find(|(id, _)| *id == tool) else {
            continue;
        };
        if clean.iter().any(|(id, _)| id == tool) {
            continue;
        }
        let key = key.trim().to_ascii_uppercase();
        let key = if valid_editor_key(&key) {
            key
        } else {
            (*fallback).into()
        };
        let key = if !key.is_empty() && clean.iter().any(|(_, k)| *k == key) {
            String::new()
        } else {
            key
        };
        clean.push((tool.clone(), key));
    }
    for (tool, key) in EDITOR_SHORTCUTS {
        if !clean.iter().any(|(id, _)| id == tool) {
            let key = if clean.iter().any(|(_, k)| k == key) { "" } else { key };
            clean.push((tool.into(), key.into()));
        }
    }
    *keys = clean;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceLayout {
    pub ui_scale: u16,
    pub editor_zoom_speed: u16,
    pub editor_show_properties: bool,
    pub editor_tools_width: u16,
    pub editor_properties_width: u16,
    pub compact_ui: bool,
    pub editor_tools_right: bool,
    pub editor_keys: Vec<(String, String)>,
    pub overlay_keys: Vec<(OverlayTool, String)>,
    pub overlay_hidden_tools: Vec<OverlayTool>,
    pub overlay_toolbar: ToolbarPos,
    pub overlay_palette: Vec<[u8; 4]>,
}

impl Default for WorkspaceLayout {
    fn default() -> Self {
        Settings::default().workspace_layout()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceProfile {
    pub name: String,
    pub layout: WorkspaceLayout,
}

fn checked_name(name: &str, profiles: &[WorkspaceProfile], except: Option<usize>) -> Result<String, &'static str> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
        return Err("name");
    }
    if profiles
        .iter()
        .enumerate()
        .any(|(i, p)| Some(i) != except && p.name.to_lowercase() == name.to_lowercase())
    {
        return Err("duplicate");
    }
    Ok(name.to_owned())
}

impl Settings {
    pub fn workspace_layout(&self) -> WorkspaceLayout {
        WorkspaceLayout {
            ui_scale: self.ui_scale,
            editor_zoom_speed: self.editor_zoom_speed,
            editor_show_properties: self.editor_show_properties,
            editor_tools_width: self.editor_tools_width,
            editor_properties_width: self.editor_properties_width,
            compact_ui: self.compact_ui,
            editor_tools_right: self.editor_tools_right,
            editor_keys: self.editor_keys.clone(),
            overlay_keys: self.overlay_keys.clone(),
            overlay_hidden_tools: self.overlay_hidden_tools.clone(),
            overlay_toolbar: self.overlay_toolbar,
            overlay_palette: self.overlay_palette.clone(),
        }
    }
    pub fn apply_workspace_layout(&mut self, layout: &WorkspaceLayout) {
        self.ui_scale = layout.ui_scale;
        self.editor_zoom_speed = layout.editor_zoom_speed;
        self.editor_show_properties = layout.editor_show_properties;
        self.editor_tools_width = layout.editor_tools_width;
        self.editor_properties_width = layout.editor_properties_width;
        self.compact_ui = layout.compact_ui;
        self.editor_tools_right = layout.editor_tools_right;
        self.editor_keys = layout.editor_keys.clone();
        self.overlay_keys = layout.overlay_keys.clone();
        self.overlay_hidden_tools = layout.overlay_hidden_tools.clone();
        self.overlay_toolbar = layout.overlay_toolbar;
        self.overlay_palette = layout.overlay_palette.clone();
    }
    pub fn editor_key(&self, tool: &str) -> &str {
        self.editor_keys
            .iter()
            .find(|(id, _)| id == tool)
            .map_or("", |(_, key)| key)
    }
    pub fn create_workspace(&mut self, name: &str) -> Result<(), &'static str> {
        let name = checked_name(name, &self.workspace_profiles, None)?;
        if self.workspace_profiles.len() >= 32 {
            return Err("limit");
        }
        self.workspace_profiles.push(WorkspaceProfile {
            name,
            layout: self.workspace_layout(),
        });
        self.active_workspace = Some(self.workspace_profiles.len() - 1);
        Ok(())
    }
    pub fn activate_workspace(&mut self, index: usize) -> Result<(), &'static str> {
        let layout = self.workspace_profiles.get(index).ok_or("missing")?.layout.clone();
        self.apply_workspace_layout(&layout);
        self.active_workspace = Some(index);
        Ok(())
    }
    pub fn save_workspace(&mut self) -> Result<(), &'static str> {
        let index = self.active_workspace.ok_or("missing")?;
        let layout = self.workspace_layout();
        self.workspace_profiles.get_mut(index).ok_or("missing")?.layout = layout;
        Ok(())
    }
    pub fn rename_workspace(&mut self, name: &str) -> Result<(), &'static str> {
        let index = self.active_workspace.ok_or("missing")?;
        let name = checked_name(name, &self.workspace_profiles, Some(index))?;
        self.workspace_profiles.get_mut(index).ok_or("missing")?.name = name;
        Ok(())
    }
    pub fn remove_workspace(&mut self) -> Result<(), &'static str> {
        let index = self
            .active_workspace
            .filter(|i| *i < self.workspace_profiles.len())
            .ok_or("missing")?;
        self.workspace_profiles.remove(index);
        self.active_workspace = None;
        Ok(())
    }
    pub fn sanitize_workspaces(&mut self) {
        self.workspace_profiles.truncate(32);
        let old_len = self.workspace_profiles.len();
        let mut clean = Vec::new();
        for mut profile in std::mem::take(&mut self.workspace_profiles) {
            if let Ok(name) = checked_name(&profile.name, &clean, None) {
                profile.name = name;
                let mut settings = Settings::default();
                settings.apply_workspace_layout(&profile.layout);
                settings.sanitize();
                profile.layout = settings.workspace_layout();
                clean.push(profile);
            }
        }
        if clean.len() != old_len {
            self.active_workspace = None;
        }
        self.workspace_profiles = clean;
        self.active_workspace = self.active_workspace.filter(|i| *i < self.workspace_profiles.len());
    }
}
