use crate::tokens::{LayoutMetrics, ThemePalette, FONT_MONO_BOLD, FONT_UI_BOLD, FONT_UI_SEMIBOLD};
use iced::widget::{button, column, container, row, text, text_input, Column, Container};
use iced::{Alignment, Background, Border, Color, Element, Length};

pub fn primary_button_style(
    palette: ThemePalette,
    radius: f32,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => palette.accent_hover,
            button::Status::Disabled => palette.bg_hover,
            button::Status::Active => palette.accent,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: palette.on_accent,
            border: Border {
                radius: radius.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub fn secondary_button_style(
    palette: ThemePalette,
    radius: f32,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => palette.bg_hover,
            _ => palette.bg_elevated,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: palette.text_primary,
            border: Border {
                color: palette.border_subtle,
                width: 1.0,
                radius: radius.into(),
            },
            ..Default::default()
        }
    }
}

pub fn nav_pill_button_style(
    palette: ThemePalette,
    radius: f32,
    active: bool,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = if active {
            Some(Background::Color(palette.accent_soft))
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(Background::Color(palette.bg_hover))
        } else {
            None
        };
        let border = if active {
            Border {
                color: Color {
                    a: 0.35,
                    ..palette.accent
                },
                width: 1.0,
                radius: radius.into(),
            }
        } else {
            Border {
                radius: radius.into(),
                ..Default::default()
            }
        };
        button::Style {
            background: bg,
            text_color: if active {
                palette.accent
            } else {
                palette.text_primary
            },
            border,
            ..Default::default()
        }
    }
}

pub fn ghost_button_style(
    palette: ThemePalette,
    radius: f32,
    active: bool,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = if active {
            Some(Background::Color(palette.bg_elevated))
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(Background::Color(palette.bg_hover))
        } else {
            None
        };
        button::Style {
            background: bg,
            text_color: if active {
                palette.accent
            } else {
                palette.text_primary
            },
            border: Border {
                radius: radius.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub fn chip_button_style(
    palette: ThemePalette,
    active: bool,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (bg, border_color, text_color) = if active {
            (palette.accent, palette.accent, palette.on_accent)
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            (palette.bg_hover, palette.text_muted, palette.text_primary)
        } else {
            (palette.bg_elevated, palette.border_subtle, palette.text_secondary)
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color,
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 999.0.into(),
            },
            ..Default::default()
        }
    }
}

pub fn danger_button_style(
    palette: ThemePalette,
    radius: f32,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => Color {
                a: 0.85,
                ..palette.danger
            },
            _ => palette.danger,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: Color::WHITE,
            border: Border {
                radius: radius.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub fn input_style(
    palette: ThemePalette,
    radius: f32,
) -> impl Fn(&iced::Theme, text_input::Status) -> text_input::Style {
    move |_theme, status| {
        let border_color = match status {
            text_input::Status::Focused => palette.accent,
            text_input::Status::Hovered => palette.text_muted,
            _ => palette.border_subtle,
        };
        text_input::Style {
            background: Background::Color(palette.bg_elevated),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: radius.into(),
            },
            icon: palette.text_muted,
            placeholder: palette.text_muted,
            value: palette.text_primary,
            selection: Color {
                a: 0.35,
                ..palette.accent
            },
        }
    }
}

pub fn panel_style(palette: ThemePalette, bg: Color, radius: f32) -> container::Style {
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: palette.border_subtle,
            width: 1.0,
            radius: radius.into(),
        },
        text_color: Some(self::text_color(palette)),
        ..Default::default()
    }
}

fn text_color(palette: ThemePalette) -> Color {
    palette.text_primary
}

/// Circular monogram avatar used for channels and user accounts.
pub fn avatar_circle<'a, Message: 'a>(
    label: &str,
    bg_color: Color,
    size: f32,
) -> Container<'a, Message> {
    let ch = label
        .trim()
        .trim_start_matches('@')
        .chars()
        .next()
        .unwrap_or('F')
        .to_uppercase()
        .to_string();
    container(
        text(ch)
            .font(FONT_UI_BOLD)
            .size((size * 0.44).max(11.0))
            .color(Color::WHITE),
    )
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .center_x(Length::Fixed(size))
    .center_y(Length::Fixed(size))
    .style(move |_| container::Style {
        background: Some(Background::Color(bg_color)),
        border: Border {
            radius: (size / 2.0).into(),
            ..Default::default()
        },
        ..Default::default()
    })
}

/// Deterministic pleasant avatar color derived from channel or account name.
pub fn deterministic_tint(seed: &str) -> Color {
    const PALETTE: [(u8, u8, u8); 6] = [
        (239, 68, 68),
        (59, 130, 246),
        (16, 185, 129),
        (168, 85, 247),
        (245, 158, 11),
        (236, 72, 153),
    ];
    let hash = seed.bytes().fold(0usize, |acc, b| acc.wrapping_add(b as usize));
    let (r, g, b) = PALETTE[hash % PALETTE.len()];
    Color::from_rgb8(r, g, b)
}

pub fn pill_badge<'a, Message: 'a>(
    label: impl Into<String>,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Container<'a, Message> {
    container(
        text(label.into())
            .font(FONT_UI_SEMIBOLD)
            .size(metrics.font_micro)
            .color(palette.text_secondary),
    )
    .padding([4, 10])
    .style(move |_| container::Style {
        background: Some(Background::Color(palette.bg_elevated)),
        border: Border {
            color: palette.border_subtle,
            width: 1.0,
            radius: 999.0.into(),
        },
        ..Default::default()
    })
}

pub fn accent_badge<'a, Message: 'a>(
    label: impl Into<String>,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Container<'a, Message> {
    container(
        text(label.into())
            .font(FONT_UI_SEMIBOLD)
            .size(metrics.font_micro)
            .color(palette.accent),
    )
    .padding([4, 10])
    .style(move |_| container::Style {
        background: Some(Background::Color(palette.accent_soft)),
        border: Border {
            color: Color {
                a: 0.35,
                ..palette.accent
            },
            width: 1.0,
            radius: 999.0.into(),
        },
        ..Default::default()
    })
}

pub fn activation_code_box<'a, Message: 'a>(
    code: impl Into<String>,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Container<'a, Message> {
    container(
        text(code.into())
            .font(FONT_MONO_BOLD)
            .size(34.0 * metrics.scale)
            .color(palette.text_primary),
    )
    .padding([16, 28])
    .width(Length::Fill)
    .center_x(Length::Fill)
    .style(move |_| container::Style {
        background: Some(Background::Color(palette.bg_primary)),
        border: Border {
            color: palette.accent,
            width: 1.5,
            radius: metrics.radius_md.into(),
        },
        ..Default::default()
    })
}

pub fn stat_tile<'a, Message: 'a>(
    label: impl Into<String>,
    value: impl Into<String>,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Container<'a, Message> {
    container(
        column![
            text(label.into())
                .size(metrics.font_caption)
                .color(palette.text_muted),
            text(value.into())
                .font(FONT_UI_SEMIBOLD)
                .size(metrics.font_heading)
                .color(palette.text_primary),
        ]
        .spacing(4),
    )
    .padding(metrics.space_md as u16)
    .width(Length::Fill)
    .style(move |_| panel_style(palette, palette.surface_card, metrics.radius_md))
}

pub fn empty_state<'a, Message: 'a>(
    title: impl Into<String>,
    subtitle: impl Into<String>,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, Message> {
    container(
        Column::new()
            .spacing(metrics.space_sm)
            .align_x(Alignment::Center)
            .push(
                text(title.into())
                    .font(FONT_UI_SEMIBOLD)
                    .size(metrics.font_heading)
                    .color(palette.text_primary),
            )
            .push(
                text(subtitle.into())
                    .size(metrics.font_body)
                    .color(palette.text_secondary),
            ),
    )
    .width(Length::Fill)
    .padding(metrics.space_lg as u16 * 2)
    .center_x(Length::Fill)
    .into()
}

pub fn error_banner<'a, Message: Clone + 'a>(
    message: impl Into<String>,
    retry_label: Option<String>,
    on_retry: Option<Message>,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, Message> {
    let mut r = row![
        text(message.into())
            .size(metrics.font_body)
            .color(palette.danger)
            .width(Length::Fill),
    ]
    .spacing(metrics.space_sm)
    .align_y(Alignment::Center);

    if let (Some(lbl), Some(msg)) = (retry_label, on_retry) {
        r = r.push(
            button(text(lbl).size(metrics.font_caption))
                .on_press(msg)
                .padding([6, 12])
                .style(danger_button_style(palette, metrics.radius_sm)),
        );
    }

    container(r)
        .padding(metrics.space_md as u16)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color {
                a: 0.12,
                ..palette.danger
            })),
            border: Border {
                color: palette.danger,
                width: 1.0,
                radius: metrics.radius_md.into(),
            },
            ..Default::default()
        })
        .into()
}

pub fn toast_banner<'a, Message: Clone + 'a>(
    message: impl Into<String>,
    is_error: bool,
    on_dismiss: Message,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, Message> {
    let accent = if is_error {
        palette.danger
    } else {
        palette.success
    };
    container(
        row![
            text(message.into())
                .size(metrics.font_caption)
                .color(palette.text_primary)
                .width(Length::Fill),
            button(text("✕").size(metrics.font_caption))
                .on_press(on_dismiss)
                .padding([2, 8])
                .style(ghost_button_style(palette, metrics.radius_sm, false)),
        ]
        .spacing(metrics.space_sm)
        .align_y(Alignment::Center),
    )
    .padding([8, 12])
    .style(move |_| container::Style {
        background: Some(Background::Color(palette.bg_elevated)),
        border: Border {
            color: accent,
            width: 1.0,
            radius: metrics.radius_sm.into(),
        },
        ..Default::default()
    })
    .into()
}
