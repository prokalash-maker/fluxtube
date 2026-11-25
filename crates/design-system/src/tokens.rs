use iced::{Background, Border, Color};
use serde::{Deserialize, Serialize};

/// Supported FluxTube UI theme presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeKind {
    GraphiteDark,
    PureBlack,
    ArcticLight,
    System,
    Custom,
}

impl ThemeKind {
    pub fn from_key(key: &str) -> Self {
        match key {
            "graphite_dark" => Self::GraphiteDark,
            "pure_black" => Self::PureBlack,
            "arctic_light" => Self::ArcticLight,
            "system" => Self::System,
            "custom" => Self::Custom,
            _ => Self::GraphiteDark,
        }
    }

    pub fn as_key(&self) -> &'static str {
        match self {
            Self::GraphiteDark => "graphite_dark",
            Self::PureBlack => "pure_black",
            Self::ArcticLight => "arctic_light",
            Self::System => "system",
            Self::Custom => "custom",
        }
    }
}

/// UI layout density mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DensityMode {
    Compact,
    Standard,
    Comfortable,
}

impl DensityMode {
    pub fn from_key(key: &str) -> Self {
        match key {
            "compact" => Self::Compact,
            "standard" => Self::Standard,
            "comfortable" => Self::Comfortable,
            _ => Self::Standard,
        }
    }

    pub fn as_key(&self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Standard => "standard",
            Self::Comfortable => "comfortable",
        }
    }
}

/// Semantic color tokens for the entire FluxTube interface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemePalette {
    pub is_dark: bool,
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_elevated: Color,
    pub bg_hover: Color,
    pub surface_card: Color,
    pub border_subtle: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub on_accent: Color,
    pub danger: Color,
    pub success: Color,
    pub warning: Color,
}

/// Parses a `#RRGGBB` hex string into an Iced `Color`.
pub fn parse_hex_color(hex_str: &str) -> Option<Color> {
    let s = hex_str.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::from_rgb8(r, g, b))
}

fn lighten(c: Color, delta: f32) -> Color {
    Color::from_rgba(
        (c.r + delta).min(1.0),
        (c.g + delta).min(1.0),
        (c.b + delta).min(1.0),
        c.a,
    )
}

impl ThemePalette {
    pub fn resolve(kind: ThemeKind, accent_hex: &str, system_dark: bool) -> Self {
        let accent = parse_hex_color(accent_hex).unwrap_or_else(|| Color::from_rgb8(255, 0, 51));
        let accent_hover = lighten(accent, 0.08);

        let effective = match kind {
            ThemeKind::System => {
                if system_dark {
                    ThemeKind::GraphiteDark
                } else {
                    ThemeKind::ArcticLight
                }
            }
            other => other,
        };

        match effective {
            ThemeKind::GraphiteDark => Self {
                is_dark: true,
                bg_primary: Color::from_rgb8(24, 24, 27),
                bg_secondary: Color::from_rgb8(31, 31, 35),
                bg_elevated: Color::from_rgb8(39, 39, 44),
                bg_hover: Color::from_rgb8(49, 49, 56),
                surface_card: Color::from_rgb8(34, 34, 39),
                border_subtle: Color::from_rgb8(56, 56, 64),
                text_primary: Color::from_rgb8(244, 244, 245),
                text_secondary: Color::from_rgb8(180, 180, 190),
                text_muted: Color::from_rgb8(128, 128, 138),
                accent,
                accent_hover,
                on_accent: Color::WHITE,
                danger: Color::from_rgb8(239, 68, 68),
                success: Color::from_rgb8(34, 197, 94),
                warning: Color::from_rgb8(245, 158, 11),
            },
            ThemeKind::PureBlack => Self {
                is_dark: true,
                bg_primary: Color::from_rgb8(0, 0, 0),
                bg_secondary: Color::from_rgb8(12, 12, 14),
                bg_elevated: Color::from_rgb8(20, 20, 23),
                bg_hover: Color::from_rgb8(32, 32, 36),
                surface_card: Color::from_rgb8(16, 16, 19),
                border_subtle: Color::from_rgb8(38, 38, 42),
                text_primary: Color::from_rgb8(250, 250, 250),
                text_secondary: Color::from_rgb8(170, 170, 178),
                text_muted: Color::from_rgb8(115, 115, 125),
                accent,
                accent_hover,
                on_accent: Color::WHITE,
                danger: Color::from_rgb8(248, 113, 113),
                success: Color::from_rgb8(74, 222, 128),
                warning: Color::from_rgb8(251, 191, 36),
            },
            ThemeKind::ArcticLight => Self {
                is_dark: false,
                bg_primary: Color::from_rgb8(247, 248, 250),
                bg_secondary: Color::from_rgb8(239, 241, 245),
                bg_elevated: Color::from_rgb8(255, 255, 255),
                bg_hover: Color::from_rgb8(229, 233, 240),
                surface_card: Color::from_rgb8(255, 255, 255),
                border_subtle: Color::from_rgb8(216, 222, 233),
                text_primary: Color::from_rgb8(24, 28, 36),
                text_secondary: Color::from_rgb8(76, 86, 106),
                text_muted: Color::from_rgb8(120, 130, 148),
                accent,
                accent_hover,
                on_accent: Color::WHITE,
                danger: Color::from_rgb8(220, 38, 38),
                success: Color::from_rgb8(22, 163, 74),
                warning: Color::from_rgb8(217, 119, 6),
            },
            ThemeKind::Custom => Self {
                is_dark: true,
                bg_primary: Color::from_rgb8(18, 22, 28),
                bg_secondary: Color::from_rgb8(24, 29, 38),
                bg_elevated: Color::from_rgb8(31, 38, 49),
                bg_hover: Color::from_rgb8(42, 51, 66),
                surface_card: Color::from_rgb8(26, 32, 42),
                border_subtle: Color::from_rgb8(50, 60, 76),
                text_primary: Color::from_rgb8(241, 245, 249),
                text_secondary: Color::from_rgb8(176, 190, 208),
                text_muted: Color::from_rgb8(120, 136, 158),
                accent,
                accent_hover,
                on_accent: Color::WHITE,
                danger: Color::from_rgb8(239, 68, 68),
                success: Color::from_rgb8(34, 197, 94),
                warning: Color::from_rgb8(245, 158, 11),
            },
            ThemeKind::System => unreachable!(),
        }
    }

    pub fn card_container_style(&self, radius: f32) -> iced::widget::container::Style {
        iced::widget::container::Style {
            background: Some(Background::Color(self.surface_card)),
            border: Border {
                color: self.border_subtle,
                width: 1.0,
                radius: radius.into(),
            },
            text_color: Some(self.text_primary),
            ..Default::default()
        }
    }
}

/// Spacing and typography scale adjusted by density and user UI scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutMetrics {
    pub scale: f32,
    pub density: DensityMode,
    pub space_xs: f32,
    pub space_sm: f32,
    pub space_md: f32,
    pub space_lg: f32,
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,
    pub font_micro: f32,
    pub font_caption: f32,
    pub font_body: f32,
    pub font_heading: f32,
    pub font_title: f32,
    pub sidebar_expanded_width: f32,
    pub sidebar_collapsed_width: f32,
    pub card_min_width: f32,
}

impl LayoutMetrics {
    pub fn new(density: DensityMode, scale: f32) -> Self {
        let s = scale.clamp(0.75, 1.5);
        let d_mult = match density {
            DensityMode::Compact => 0.85,
            DensityMode::Standard => 1.0,
            DensityMode::Comfortable => 1.2,
        };
        let card_min = match density {
            DensityMode::Compact => 230.0 * s,
            DensityMode::Standard => 270.0 * s,
            DensityMode::Comfortable => 310.0 * s,
        };

        Self {
            scale: s,
            density,
            space_xs: 4.0 * d_mult * s,
            space_sm: 8.0 * d_mult * s,
            space_md: 14.0 * d_mult * s,
            space_lg: 22.0 * d_mult * s,
            radius_sm: 6.0 * s,
            radius_md: 10.0 * s,
            radius_lg: 14.0 * s,
            font_micro: 11.0 * s,
            font_caption: 12.5 * s,
            font_body: 14.0 * s,
            font_heading: 17.5 * s,
            font_title: 22.0 * s,
            sidebar_expanded_width: 224.0 * s,
            sidebar_collapsed_width: 64.0 * s,
            card_min_width: card_min,
        }
    }

    /// Computes responsive grid columns for a given content area width (1280x720 -> 4K).
    pub fn grid_columns(&self, content_width: f32) -> usize {
        let available = content_width.max(300.0);
        let cols = ((available + self.space_md) / (self.card_min_width + self.space_md)).floor() as usize;
        cols.clamp(1, 6)
    }
}
