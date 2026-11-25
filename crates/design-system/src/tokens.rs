use iced::font::{Family, Weight};
use iced::{Background, Border, Color, Font};
use serde::{Deserialize, Serialize};

pub const FONT_UI_SEMIBOLD: Font = Font {
    family: Family::SansSerif,
    weight: Weight::Semibold,
    ..Font::DEFAULT
};

pub const FONT_UI_BOLD: Font = Font {
    family: Family::SansSerif,
    weight: Weight::Bold,
    ..Font::DEFAULT
};

pub const FONT_MONO_BOLD: Font = Font {
    family: Family::Monospace,
    weight: Weight::Bold,
    ..Font::MONOSPACE
};

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
    pub accent_soft: Color,
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

fn with_alpha(c: Color, alpha: f32) -> Color {
    Color { a: alpha, ..c }
}

impl ThemePalette {
    pub fn resolve(kind: ThemeKind, accent_hex: &str, system_dark: bool) -> Self {
        let accent = parse_hex_color(accent_hex).unwrap_or_else(|| Color::from_rgb8(255, 45, 85));
        let accent_hover = lighten(accent, 0.08);
        let accent_soft = with_alpha(accent, 0.14);

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
                bg_primary: Color::from_rgb8(17, 18, 22),
                bg_secondary: Color::from_rgb8(23, 25, 30),
                bg_elevated: Color::from_rgb8(31, 34, 41),
                bg_hover: Color::from_rgb8(41, 45, 55),
                surface_card: Color::from_rgb8(26, 28, 34),
                border_subtle: Color::from_rgb8(46, 50, 61),
                text_primary: Color::from_rgb8(244, 245, 248),
                text_secondary: Color::from_rgb8(178, 183, 196),
                text_muted: Color::from_rgb8(122, 128, 144),
                accent,
                accent_hover,
                accent_soft,
                on_accent: Color::WHITE,
                danger: Color::from_rgb8(239, 68, 68),
                success: Color::from_rgb8(34, 197, 94),
                warning: Color::from_rgb8(245, 158, 11),
            },
            ThemeKind::PureBlack => Self {
                is_dark: true,
                bg_primary: Color::BLACK,
                bg_secondary: Color::from_rgb8(10, 10, 13),
                bg_elevated: Color::from_rgb8(19, 20, 24),
                bg_hover: Color::from_rgb8(31, 33, 39),
                surface_card: Color::from_rgb8(14, 15, 18),
                border_subtle: Color::from_rgb8(36, 38, 44),
                text_primary: Color::from_rgb8(250, 250, 252),
                text_secondary: Color::from_rgb8(172, 176, 188),
                text_muted: Color::from_rgb8(116, 120, 132),
                accent,
                accent_hover,
                accent_soft,
                on_accent: Color::WHITE,
                danger: Color::from_rgb8(248, 113, 113),
                success: Color::from_rgb8(74, 222, 128),
                warning: Color::from_rgb8(251, 191, 36),
            },
            ThemeKind::ArcticLight => Self {
                is_dark: false,
                bg_primary: Color::from_rgb8(245, 246, 249),
                bg_secondary: Color::from_rgb8(255, 255, 255),
                bg_elevated: Color::from_rgb8(241, 243, 248),
                bg_hover: Color::from_rgb8(231, 235, 242),
                surface_card: Color::from_rgb8(255, 255, 255),
                border_subtle: Color::from_rgb8(221, 226, 235),
                text_primary: Color::from_rgb8(19, 23, 32),
                text_secondary: Color::from_rgb8(75, 85, 105),
                text_muted: Color::from_rgb8(120, 130, 148),
                accent,
                accent_hover,
                accent_soft,
                on_accent: Color::WHITE,
                danger: Color::from_rgb8(220, 38, 38),
                success: Color::from_rgb8(22, 163, 74),
                warning: Color::from_rgb8(217, 119, 6),
            },
            ThemeKind::Custom => Self {
                is_dark: true,
                bg_primary: Color::from_rgb8(15, 20, 28),
                bg_secondary: Color::from_rgb8(21, 27, 37),
                bg_elevated: Color::from_rgb8(29, 37, 50),
                bg_hover: Color::from_rgb8(40, 50, 67),
                surface_card: Color::from_rgb8(24, 31, 42),
                border_subtle: Color::from_rgb8(48, 59, 77),
                text_primary: Color::from_rgb8(242, 246, 252),
                text_secondary: Color::from_rgb8(178, 192, 212),
                text_muted: Color::from_rgb8(120, 136, 160),
                accent,
                accent_hover,
                accent_soft,
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
            DensityMode::Compact => 240.0 * s,
            DensityMode::Standard => 275.0 * s,
            DensityMode::Comfortable => 315.0 * s,
        };

        Self {
            scale: s,
            density,
            space_xs: 4.0 * d_mult * s,
            space_sm: 8.0 * d_mult * s,
            space_md: 14.0 * d_mult * s,
            space_lg: 22.0 * d_mult * s,
            radius_sm: 8.0 * s,
            radius_md: 12.0 * s,
            radius_lg: 16.0 * s,
            font_micro: 11.0 * s,
            font_caption: 12.5 * s,
            font_body: 14.0 * s,
            font_heading: 17.0 * s,
            font_title: 22.0 * s,
            sidebar_expanded_width: 228.0 * s,
            sidebar_collapsed_width: 68.0 * s,
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
