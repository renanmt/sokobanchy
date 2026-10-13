use eframe::egui::{self, Color32};
use std::{env, fs, path::PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: String,
    pub background: Color32,
    pub panel: Color32,
    pub surface: Color32,
    pub foreground: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub green: Color32,
    pub yellow: Color32,
    pub red: Color32,
    pub light: bool,
}
pub fn xdg(var: &str, fallback: &str) -> PathBuf {
    env::var_os(var)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            PathBuf::from(env::var_os("HOME").unwrap_or_else(|| ".".into())).join(fallback)
        })
}
pub fn color(s: &str) -> Option<Color32> {
    let s = s.strip_prefix('#')?;
    if s.len() != 6 || !s.is_ascii() {
        return None;
    }
    let n = u32::from_str_radix(s, 16).ok()?;
    Some(Color32::from_rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "Tokyo Night · fallback".into(),
            background: Color32::from_rgb(26, 27, 38),
            panel: Color32::from_rgb(19, 20, 28),
            surface: Color32::from_rgb(36, 40, 59),
            foreground: Color32::from_rgb(192, 202, 245),
            muted: Color32::from_rgb(122, 132, 166),
            accent: Color32::from_rgb(122, 162, 247),
            green: Color32::from_rgb(158, 206, 106),
            yellow: Color32::from_rgb(224, 175, 104),
            red: Color32::from_rgb(247, 118, 142),
            light: false,
        }
    }
}
impl Theme {
    pub fn parse(text: &str, name: String) -> Option<Self> {
        let doc: toml::Value = toml::from_str(text).ok()?;
        let mut t = Self {
            name,
            ..Self::default()
        };
        t.light = doc.get("mode").and_then(|v| v.as_str()) == Some("light");
        for (keys, field) in [
            (&["background"][..], &mut t.background),
            (&["dark_background", "background"][..], &mut t.panel),
            (&["lighter_background", "selection"][..], &mut t.surface),
            (&["foreground"][..], &mut t.foreground),
            (&["dark_foreground", "foreground"][..], &mut t.muted),
            (&["accent", "color4"][..], &mut t.accent),
            (&["green", "color2"][..], &mut t.green),
            (&["yellow", "color3"][..], &mut t.yellow),
            (&["red", "color1"][..], &mut t.red),
        ] {
            if let Some(c) = keys
                .iter()
                .find_map(|k| doc.get(*k).and_then(|v| v.as_str()).and_then(color))
            {
                *field = c;
            }
        }
        // Legacy palettes omit mode; infer it from the background.
        if doc.get("mode").is_none() {
            t.light =
                (t.background.r() as u32 + t.background.g() as u32 + t.background.b() as u32) > 450;
        }
        Some(t)
    }
    pub fn load(explicit: Option<&std::path::Path>) -> Self {
        let candidates = explicit.map(|p| vec![p.to_path_buf()]).unwrap_or_else(|| {
            vec![
                xdg("XDG_STATE_HOME", ".local/state").join("omarchy/current/theme/colors.toml"),
                xdg("XDG_CONFIG_HOME", ".config").join("omarchy/current/theme/colors.toml"),
            ]
        });
        for path in candidates {
            if let Ok(text) = fs::read_to_string(&path) {
                let name_path = path
                    .parent()
                    .and_then(|p| p.parent())
                    .map(|p| p.join("theme.name"));
                let name = name_path
                    .and_then(|p| fs::read_to_string(p).ok())
                    .map(|s| s.trim().replace('-', " "))
                    .unwrap_or_else(|| {
                        if explicit.is_some() {
                            "Custom palette".into()
                        } else {
                            "Omarchy".into()
                        }
                    });
                if let Some(theme) = Self::parse(&text, name) {
                    return theme;
                }
            }
        }
        Self::default()
    }
    pub fn apply(&self, ctx: &egui::Context) {
        let mut v = if self.light {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };
        v.override_text_color = Some(self.foreground);
        v.panel_fill = self.background;
        v.window_fill = self.panel;
        v.extreme_bg_color = self.panel;
        v.faint_bg_color = self.surface;
        v.selection.bg_fill = self.accent;
        v.selection.stroke.color = self.background;
        for w in [
            &mut v.widgets.inactive,
            &mut v.widgets.active,
            &mut v.widgets.hovered,
        ] {
            w.bg_fill = self.surface;
            w.weak_bg_fill = self.surface;
            w.fg_stroke.color = self.foreground;
            w.corner_radius = egui::CornerRadius::same(3);
        }
        v.widgets.hovered.bg_stroke.color = self.accent;
        v.widgets.active.bg_fill = self.accent;
        ctx.set_visuals(v);
    }
}
