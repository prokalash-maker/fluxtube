use crate::messages::{AuthTab, ChannelTab, Route, SettingsTab, UiMessage};
use crate::state::{format_relative_age, format_seconds_hms, UiState};
use fluxtube_core::{PerformanceProfileType, Video, VideoRating};
use fluxtube_design_system::{
    accent_badge, activation_code_box, avatar_circle, chip_button_style, deterministic_tint,
    empty_state, error_banner, ghost_button_style, input_style, nav_pill_button_style,
    panel_style, pill_badge, primary_button_style, secondary_button_style, stat_tile, toast_banner,
    DensityMode, LayoutMetrics, ThemeKind, ThemePalette, FONT_MONO_BOLD, FONT_UI_BOLD,
    FONT_UI_SEMIBOLD,
};
use fluxtube_localization::LocalizationManager;
use fluxtube_youtube_api::{SearchOrder, SearchResourceType, VideoDurationFilter};
use iced::widget::{
    button, checkbox, column, container, image, row, scrollable, slider, stack, text, text_input,
    Column, Row, Space,
};
use iced::{Alignment, Background, Border, Color, Element, Length};

pub fn route_label(route: Route, loc: &LocalizationManager) -> String {
    match route {
        Route::Home => loc.t("nav.home"),
        Route::Trending => loc.t("nav.trending"),
        Route::Subscriptions => loc.t("nav.subscriptions"),
        Route::Shorts => loc.t("nav.shorts"),
        Route::Live => loc.t("nav.live"),
        Route::History => loc.t("nav.history"),
        Route::WatchLater => loc.t("nav.watch_later"),
        Route::Liked => loc.t("nav.liked"),
        Route::Playlists => loc.t("nav.playlists"),
        Route::Favorites => loc.t("nav.favorites"),
        Route::Settings => loc.t("nav.settings"),
        Route::Diagnostics => loc.t("nav.diagnostics"),
        Route::Search => loc.t("search.button"),
        Route::Watch => loc.t("app.name"),
        Route::Channel => loc.t("channel.videos"),
    }
}

fn route_icon(route: Route) -> &'static str {
    match route {
        Route::Home => "⌂",
        Route::Trending => "▲",
        Route::Subscriptions => "◈",
        Route::Shorts => "⚡",
        Route::Live => "●",
        Route::History => "◷",
        Route::WatchLater => "◫",
        Route::Liked => "♥",
        Route::Playlists => "☰",
        Route::Favorites => "★",
        Route::Settings => "⚙",
        Route::Diagnostics => "⌘",
        Route::Search => "🔍",
        Route::Watch => "▶",
        Route::Channel => "◉",
    }
}

pub fn render_app<'a>(state: &'a UiState, loc: &'a LocalizationManager) -> Element<'a, UiMessage> {
    let palette = state.palette();
    let metrics = state.metrics();

    let mut root_col = Column::new().width(Length::Fill).height(Length::Fill);

    if !state.fullscreen_mode {
        root_col = root_col.push(render_top_bar(state, loc, palette, metrics));
    }

    let main_content = render_main_content(state, loc, palette, metrics);

    let body: Element<'a, UiMessage> = if state.fullscreen_mode {
        main_content
    } else {
        row![render_sidebar(state, loc, palette, metrics), main_content]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    root_col = root_col.push(body);

    if state.mini_player_active && state.route != Route::Watch {
        if let Some(ref vid) = state.current_video {
            root_col = root_col.push(render_mini_player_bar(state, vid, loc, palette, metrics));
        }
    }

    if let Some((ref msg, is_err)) = state.toast {
        root_col = root_col.push(
            container(toast_banner(
                msg.clone(),
                is_err,
                UiMessage::DismissToast,
                palette,
                metrics,
            ))
            .padding([6, 16]),
        );
    }

    let base: Element<'a, UiMessage> = container(root_col)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(palette.bg_primary)),
            text_color: Some(palette.text_primary),
            ..Default::default()
        })
        .into();

    if state.show_account_modal {
        stack![base, render_account_modal(state, loc, palette, metrics)].into()
    } else {
        base
    }
}

fn render_top_bar<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let search_input = text_input(&loc.t("search.placeholder"), &state.search_query)
        .on_input(UiMessage::SearchInputChanged)
        .on_submit(UiMessage::SubmitSearch)
        .padding([8, 14])
        .size(metrics.font_body)
        .width(Length::Fill)
        .style(input_style(palette, 999.0));

    let account_chip: Element<'a, UiMessage> = match &state.active_account {
        Some(acc) => {
            let tint = deterministic_tint(&acc.name);
            let avatar_widget: Element<'a, UiMessage> = match acc
                .picture
                .as_ref()
                .and_then(|url| state.thumbnails.get(url))
            {
                Some(handle) => container(
                    image(handle.clone())
                        .width(Length::Fixed(24.0))
                        .height(Length::Fixed(24.0))
                        .content_fit(iced::ContentFit::Cover),
                )
                .width(Length::Fixed(24.0))
                .height(Length::Fixed(24.0))
                .clip(true)
                .into(),
                None => avatar_circle(&acc.name, tint, 24.0).into(),
            };
            let label = if state.auth_reauth_required {
                format!("{} (!)", acc.name)
            } else {
                acc.name.clone()
            };
            button(
                row![
                    avatar_widget,
                    text(label).font(FONT_UI_SEMIBOLD).size(metrics.font_caption),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .on_press(UiMessage::ToggleAccountModal(!state.show_account_modal))
            .padding([5, 12])
            .style(secondary_button_style(palette, 999.0))
            .into()
        }
        None => button(
            row![
                avatar_circle("G", palette.accent, 22.0),
                text(loc.t("auth.login"))
                    .font(FONT_UI_SEMIBOLD)
                    .size(metrics.font_caption),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .on_press(UiMessage::ToggleAccountModal(!state.show_account_modal))
        .padding([5, 12])
        .style(secondary_button_style(palette, 999.0))
        .into(),
    };

    let brand_badge = button(
        row![
            container(text("▶").size(12).color(Color::WHITE))
                .width(Length::Fixed(24.0))
                .height(Length::Fixed(24.0))
                .center_x(Length::Fixed(24.0))
                .center_y(Length::Fixed(24.0))
                .style(move |_| container::Style {
                    background: Some(Background::Color(palette.accent)),
                    border: Border {
                        radius: 7.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            text(loc.t("app.name"))
                .font(FONT_UI_BOLD)
                .size(metrics.font_heading)
                .color(palette.text_primary),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .on_press(UiMessage::Navigate(Route::Home))
    .padding([4, 8])
    .style(ghost_button_style(palette, metrics.radius_sm, false));

    let mut bar = row![
        button(
            text(if state.sidebar_collapsed { "☰" } else { "◁" })
                .size(metrics.font_body)
                .color(palette.text_secondary)
        )
        .on_press(UiMessage::ToggleSidebar)
        .padding([6, 10])
        .style(ghost_button_style(palette, metrics.radius_sm, false)),
        brand_badge,
        Space::with_width(Length::Fixed(16.0)),
        container(search_input).width(Length::FillPortion(3)),
        button(
            text(loc.t("search.button"))
                .font(FONT_UI_SEMIBOLD)
                .size(metrics.font_caption)
        )
        .on_press(UiMessage::SubmitSearch)
        .padding([8, 16])
        .style(primary_button_style(palette, 999.0)),
        button(text(loc.t("search.open_url")).size(metrics.font_caption))
            .on_press(UiMessage::OpenUrlOrClipboard)
            .padding([8, 12])
            .style(secondary_button_style(palette, 999.0)),
        Space::with_width(Length::FillPortion(1)),
    ]
    .spacing(metrics.space_sm)
    .align_y(Alignment::Center);

    if state.is_mock_mode {
        bar = bar.push(accent_badge(loc.t("auth.mock_mode_badge"), palette, metrics));
    }
    if state.private_mode {
        bar = bar.push(pill_badge("Private", palette, metrics));
    }

    bar = bar.push(account_chip);

    container(bar)
        .padding([10, 16])
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(palette.bg_secondary)),
            border: Border {
                color: palette.border_subtle,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn render_sidebar<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let width = if state.sidebar_collapsed {
        metrics.sidebar_collapsed_width
    } else {
        metrics.sidebar_expanded_width
    };

    let mut col = Column::new().spacing(3).padding([12, 10]);

    for item in &state.sidebar_items {
        if !item.visible {
            continue;
        }
        if state.hide_shorts && item.route == Route::Shorts {
            continue;
        }
        let is_active = state.route == item.route;

        // Subtle section dividers for clean visual rhythm
        if !state.sidebar_collapsed
            && matches!(item.route, Route::History | Route::Diagnostics)
        {
            col = col.push(Space::with_height(Length::Fixed(8.0)));
            col = col.push(
                container(Space::with_height(Length::Fixed(1.0)))
                    .width(Length::Fill)
                    .style(move |_| container::Style {
                        background: Some(Background::Color(palette.border_subtle)),
                        ..Default::default()
                    }),
            );
            col = col.push(Space::with_height(Length::Fixed(8.0)));
        }

        let label_txt = if state.sidebar_collapsed {
            route_icon(item.route).to_string()
        } else {
            format!("{}   {}", route_icon(item.route), route_label(item.route, loc))
        };

        let btn_text = if is_active {
            text(label_txt).font(FONT_UI_SEMIBOLD).size(metrics.font_body)
        } else {
            text(label_txt).size(metrics.font_body)
        };

        let btn = button(btn_text)
            .on_press(UiMessage::Navigate(item.route))
            .width(Length::Fill)
            .padding([8, 12])
            .style(nav_pill_button_style(palette, metrics.radius_sm, is_active));

        col = col.push(btn);
    }

    if !state.sidebar_collapsed && !state.subscriptions.is_empty() {
        col = col
            .push(Space::with_height(Length::Fixed(14.0)))
            .push(
                row![
                    text(loc.t("nav.subscriptions"))
                        .font(FONT_UI_SEMIBOLD)
                        .size(metrics.font_caption)
                        .color(palette.text_muted)
                        .width(Length::Fill),
                    button(
                        text(if state.compact_subscriptions { "▾" } else { "▴" })
                            .size(metrics.font_micro)
                    )
                    .on_press(UiMessage::ToggleCompactSubscriptions)
                    .padding([2, 6])
                    .style(ghost_button_style(palette, metrics.radius_sm, false)),
                ]
                .padding([0, 6])
                .align_y(Alignment::Center),
            );

        let limit = if state.compact_subscriptions { 3 } else { 8 };
        for ch in state.subscriptions.iter().take(limit) {
            let tint = deterministic_tint(&ch.title);
            col = col.push(
                button(
                    row![
                        avatar_circle(&ch.title, tint, 22.0),
                        text(&ch.title).size(metrics.font_caption),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .on_press(UiMessage::OpenChannel(ch.id.clone()))
                .width(Length::Fill)
                .padding([5, 8])
                .style(ghost_button_style(palette, metrics.radius_sm, false)),
            );
        }
    }

    container(scrollable(col))
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(palette.bg_secondary)),
            border: Border {
                color: palette.border_subtle,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn render_main_content<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let mut inner = Column::new().spacing(metrics.space_md).width(Length::Fill);

    if let Some(ref err) = state.error_banner {
        inner = inner.push(error_banner(
            err.clone(),
            Some(loc.t("home.refresh")),
            Some(UiMessage::RefreshFeed),
            palette,
            metrics,
        ));
    }

    let page: Element<'a, UiMessage> = match state.route {
        Route::Home => render_home_view(state, loc, palette, metrics),
        Route::Trending => render_video_feed_view(
            &loc.t("nav.trending"),
            &state.trending_videos,
            &loc.t("search.empty"),
            state,
            palette,
            metrics,
        ),
        Route::Subscriptions => render_subscriptions_view(state, loc, palette, metrics),
        Route::Shorts => render_shorts_view(state, loc, palette, metrics),
        Route::Live => render_live_view(state, loc, palette, metrics),
        Route::History => render_history_view(state, loc, palette, metrics),
        Route::WatchLater => render_video_feed_view(
            &loc.t("nav.watch_later"),
            &state.watch_later,
            &loc.t("library.empty_watch_later"),
            state,
            palette,
            metrics,
        ),
        Route::Liked => render_video_feed_view(
            &loc.t("nav.liked"),
            &state.liked_videos,
            &loc.t("search.empty"),
            state,
            palette,
            metrics,
        ),
        Route::Playlists => render_playlists_view(state, loc, palette, metrics),
        Route::Favorites => render_favorites_view(state, loc, palette, metrics),
        Route::Search => render_search_view(state, loc, palette, metrics),
        Route::Watch => render_watch_view(state, loc, palette, metrics),
        Route::Channel => render_channel_view(state, loc, palette, metrics),
        Route::Settings => render_settings_view(state, loc, palette, metrics),
        Route::Diagnostics => render_diagnostics_view(state, loc, palette, metrics),
    };

    inner = inner.push(page);

    container(scrollable(
        container(inner)
            .padding(metrics.space_lg as u16)
            .width(Length::Fill),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn render_video_card<'a>(
    video: &'a Video,
    state: &'a UiState,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let thumb_h = 158.0 * metrics.scale;
    let tint = deterministic_tint(&video.channel_title);

    let base_thumb: Element<'a, UiMessage> =
        if let Some(handle) = state.thumbnails.get(&video.thumbnail_url) {
            image(handle.clone())
                .width(Length::Fill)
                .height(Length::Fixed(thumb_h))
                .into()
        } else {
            // Crisp minimal artwork card with subtle tint and centered play monogram
            let thumb_bg = Color::from_rgba(tint.r, tint.g, tint.b, if palette.is_dark { 0.16 } else { 0.12 });
            container(
                column![
                    container(
                        text(if video.is_live() {
                            "●"
                        } else if video.is_short() {
                            "⚡"
                        } else {
                            "▶"
                        })
                        .size(20.0)
                        .color(Color::WHITE),
                    )
                    .width(Length::Fixed(44.0))
                    .height(Length::Fixed(44.0))
                    .center_x(Length::Fixed(44.0))
                    .center_y(Length::Fixed(44.0))
                    .style(move |_| container::Style {
                        background: Some(Background::Color(Color { a: 0.85, ..tint })),
                        border: Border {
                            radius: 22.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                ]
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fixed(thumb_h))
            .style(move |_| container::Style {
                background: Some(Background::Color(thumb_bg)),
                border: Border {
                    color: palette.border_subtle,
                    width: 1.0,
                    radius: metrics.radius_sm.into(),
                },
                ..Default::default()
            })
            .into()
        };

    // Floating duration / LIVE badge positioned in bottom-right corner
    let duration_badge_text = if video.is_live() {
        "LIVE".to_string()
    } else if video.is_short() {
        format!("SHORT {}", video.formatted_duration())
    } else {
        video.formatted_duration()
    };
    let badge_bg = if video.is_live() {
        palette.danger
    } else {
        Color::from_rgba(0.05, 0.06, 0.08, 0.86)
    };

    let overlay = container(
        container(
            text(duration_badge_text)
                .font(FONT_UI_SEMIBOLD)
                .size(11.0 * metrics.scale)
                .color(Color::WHITE),
        )
        .padding([3, 7])
        .style(move |_| container::Style {
            background: Some(Background::Color(badge_bg)),
            border: Border {
                radius: 5.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }),
    )
    .width(Length::Fill)
    .height(Length::Fixed(thumb_h))
    .padding(8)
    .align_x(iced::alignment::Horizontal::Right)
    .align_y(iced::alignment::Vertical::Bottom);

    let thumb_stack = stack![base_thumb, overlay];

    let meta_row = row![
        avatar_circle(&video.channel_title, tint, 32.0 * metrics.scale),
        column![
            text(&video.title)
                .font(FONT_UI_SEMIBOLD)
                .size(metrics.font_body)
                .color(palette.text_primary),
            text(format!(
                "{}  •  {} views  •  {}",
                video.channel_title,
                video.formatted_view_count(),
                format_relative_age(video.published_at)
            ))
            .size(metrics.font_caption)
            .color(palette.text_secondary),
        ]
        .spacing(4)
        .width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Start);

    let open_btn = button(column![thumb_stack, meta_row].spacing(10))
        .on_press(UiMessage::OpenVideo(video.id.clone()))
        .padding(8)
        .width(Length::Fill)
        .style(ghost_button_style(palette, metrics.radius_md, false));

    let actions = row![
        button(text(&video.channel_title).size(metrics.font_micro).color(palette.text_muted))
            .on_press(UiMessage::OpenChannel(video.channel_id.clone()))
            .padding([3, 6])
            .style(ghost_button_style(palette, metrics.radius_sm, false)),
        Space::with_width(Length::Fill),
        button(text("+ Queue").size(metrics.font_micro))
            .on_press(UiMessage::AddToQueue(video.id.clone()))
            .padding([3, 9])
            .style(secondary_button_style(palette, 999.0)),
        button(text("◫").size(metrics.font_micro))
            .on_press(UiMessage::ToggleWatchLater(video.id.clone()))
            .padding([3, 9])
            .style(secondary_button_style(palette, 999.0)),
    ]
    .spacing(4)
    .padding([0, 8])
    .align_y(Alignment::Center);

    container(
        column![open_btn, actions]
            .spacing(4)
            .padding(iced::Padding {
                top: 2.0,
                right: 2.0,
                bottom: 8.0,
                left: 2.0,
            }),
    )
        .width(Length::Fill)
        .style(move |_| palette.card_container_style(metrics.radius_md))
        .into()
}

fn render_video_grid<'a>(
    videos: &'a [Video],
    state: &'a UiState,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let cols = metrics.grid_columns(state.content_width());
    let mut grid = Column::new().spacing(metrics.space_md).width(Length::Fill);

    for chunk in videos.chunks(cols) {
        let mut r = Row::new().spacing(metrics.space_md).width(Length::Fill);
        for v in chunk {
            r = r.push(container(render_video_card(v, state, palette, metrics)).width(Length::FillPortion(1)));
        }
        for _ in chunk.len()..cols {
            r = r.push(Space::with_width(Length::FillPortion(1)));
        }
        grid = grid.push(r);
    }

    grid.into()
}

fn render_home_view<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let categories = ["All", "Rust", "Hardware", "Live", "Short"];

    let mut styled_cat_row = Row::new().spacing(metrics.space_sm).align_y(Alignment::Center);
    for c in categories {
        let active = state.home_category == c;
        styled_cat_row = styled_cat_row.push(
            button(
                text(c)
                    .font(if active { FONT_UI_SEMIBOLD } else { iced::Font::DEFAULT })
                    .size(metrics.font_caption),
            )
            .on_press(UiMessage::SelectHomeCategory(c.to_string()))
            .padding([6, 14])
            .style(chip_button_style(palette, active)),
        );
    }

    styled_cat_row = styled_cat_row
        .push(Space::with_width(Length::Fill))
        .push(pill_badge(loc.t("home.local_badge"), palette, metrics))
        .push(
            button(text(loc.t("home.refresh")).size(metrics.font_caption))
                .on_press(UiMessage::RefreshFeed)
                .padding([6, 14])
                .style(secondary_button_style(palette, 999.0)),
        );

    let mut col = Column::new().spacing(metrics.space_lg).push(styled_cat_row);

    if !state.continue_watching.is_empty() {
        let mut cw_row = Row::new().spacing(metrics.space_sm);
        for item in state.continue_watching.iter().take(4) {
            cw_row = cw_row.push(
                button(
                    column![
                        text(&item.title).font(FONT_UI_SEMIBOLD).size(metrics.font_caption),
                        text(format!(
                            "{} • Resume {}",
                            item.channel_title,
                            format_seconds_hms(item.playback_position_seconds)
                        ))
                        .size(metrics.font_micro)
                        .color(palette.accent),
                    ]
                    .spacing(4),
                )
                .on_press(UiMessage::OpenVideoAt(
                    item.video_id.clone(),
                    item.playback_position_seconds,
                ))
                .padding(12)
                .width(Length::FillPortion(1))
                .style(secondary_button_style(palette, metrics.radius_md)),
            );
        }
        col = col
            .push(
                text(loc.t("home.continue_watching"))
                    .font(FONT_UI_BOLD)
                    .size(metrics.font_heading),
            )
            .push(cw_row);
    }

    let matching_refs: Vec<&Video> = state
        .home_videos
        .iter()
        .filter(|v| match state.home_category.as_str() {
            "Live" => v.is_live(),
            "Short" => v.is_short(),
            "Rust" => v.title.to_lowercase().contains("rust") || v.title.to_lowercase().contains("sqlite"),
            "Hardware" => v.title.to_lowercase().contains("intel") || v.title.to_lowercase().contains("ram"),
            _ => true,
        })
        .collect();

    col = col.push(
        row![
            text(loc.t("home.local_recommendations"))
                .font(FONT_UI_BOLD)
                .size(metrics.font_heading),
            Space::with_width(Length::Fixed(8.0)),
            pill_badge(loc.t("home.local_badge"), palette, metrics),
        ]
        .align_y(Alignment::Center),
    );

    if matching_refs.is_empty() {
        col = col.push(empty_state(
            loc.t("search.empty"),
            loc.t("library.device_sync_note"),
            palette,
            metrics,
        ));
    } else {
        let cols = metrics.grid_columns(state.content_width());
        let mut grid = Column::new().spacing(metrics.space_md).width(Length::Fill);
        for chunk in matching_refs.chunks(cols) {
            let mut r = Row::new().spacing(metrics.space_md).width(Length::Fill);
            for v in chunk {
                r = r.push(container(render_video_card(v, state, palette, metrics)).width(Length::FillPortion(1)));
            }
            for _ in chunk.len()..cols {
                r = r.push(Space::with_width(Length::FillPortion(1)));
            }
            grid = grid.push(r);
        }
        col = col.push(grid);
    }

    col.into()
}

fn render_video_feed_view<'a>(
    heading: &str,
    videos: &'a [Video],
    empty_msg: &str,
    state: &'a UiState,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let mut col = Column::new()
        .spacing(metrics.space_md)
        .push(text(heading.to_string()).font(FONT_UI_BOLD).size(metrics.font_title));

    if videos.is_empty() {
        col = col.push(empty_state(
            empty_msg.to_string(),
            String::new(),
            palette,
            metrics,
        ));
    } else {
        col = col.push(render_video_grid(videos, state, palette, metrics));
    }
    col.into()
}

fn render_search_view<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let type_row = row![
        text(loc.t("search.filter_type")).size(metrics.font_caption).color(palette.text_muted),
        filter_chip(loc.t("search.type_video"), state.search_type == SearchResourceType::Video, UiMessage::SetSearchType(SearchResourceType::Video), palette, metrics),
        filter_chip(loc.t("search.type_channel"), state.search_type == SearchResourceType::Channel, UiMessage::SetSearchType(SearchResourceType::Channel), palette, metrics),
        filter_chip(loc.t("search.type_playlist"), state.search_type == SearchResourceType::Playlist, UiMessage::SetSearchType(SearchResourceType::Playlist), palette, metrics),
        Space::with_width(Length::Fixed(12.0)),
        text(loc.t("search.filter_duration")).size(metrics.font_caption).color(palette.text_muted),
        filter_chip(loc.t("search.dur_any"), state.search_duration == VideoDurationFilter::Any, UiMessage::SetSearchDuration(VideoDurationFilter::Any), palette, metrics),
        filter_chip(loc.t("search.dur_short"), state.search_duration == VideoDurationFilter::Short, UiMessage::SetSearchDuration(VideoDurationFilter::Short), palette, metrics),
        filter_chip(loc.t("search.dur_medium"), state.search_duration == VideoDurationFilter::Medium, UiMessage::SetSearchDuration(VideoDurationFilter::Medium), palette, metrics),
        filter_chip(loc.t("search.dur_long"), state.search_duration == VideoDurationFilter::Long, UiMessage::SetSearchDuration(VideoDurationFilter::Long), palette, metrics),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let order_row = row![
        text(loc.t("search.filter_order")).size(metrics.font_caption).color(palette.text_muted),
        filter_chip(loc.t("search.order_relevance"), state.search_order == SearchOrder::Relevance, UiMessage::SetSearchOrder(SearchOrder::Relevance), palette, metrics),
        filter_chip(loc.t("search.order_date"), state.search_order == SearchOrder::Date, UiMessage::SetSearchOrder(SearchOrder::Date), palette, metrics),
        filter_chip(loc.t("search.order_views"), state.search_order == SearchOrder::ViewCount, UiMessage::SetSearchOrder(SearchOrder::ViewCount), palette, metrics),
        Space::with_width(Length::Fill),
        pill_badge("search.list = 100 quota units", palette, metrics),
        button(text(loc.t("search.clear_history")).size(metrics.font_micro))
            .on_press(UiMessage::ClearSearchHistory)
            .padding([5, 10])
            .style(secondary_button_style(palette, 999.0)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let mut col = Column::new()
        .spacing(metrics.space_md)
        .push(type_row)
        .push(order_row);

    if !state.search_history.is_empty() {
        let mut hist_row = Row::new().spacing(6).align_y(Alignment::Center);
        for q in state.search_history.iter().take(6) {
            hist_row = hist_row.push(
                button(text(q.clone()).size(metrics.font_micro))
                    .on_press(UiMessage::SelectSearchSuggestion(q.clone()))
                    .padding([4, 10])
                    .style(secondary_button_style(palette, 999.0)),
            );
        }
        col = col.push(hist_row);
    }

    if state.search_results.is_empty() {
        col = col.push(empty_state(
            loc.t("search.empty"),
            loc.t("search.placeholder"),
            palette,
            metrics,
        ));
    } else {
        for item in &state.search_results {
            let msg = match item.kind.as_str() {
                "youtube#channel" => UiMessage::OpenChannel(item.id.clone()),
                _ => UiMessage::OpenVideo(item.id.clone()),
            };
            let tint = deterministic_tint(&item.channel_title);
            let card = container(
                button(
                    row![
                        avatar_circle(&item.channel_title, tint, 40.0),
                        column![
                            text(&item.title).font(FONT_UI_SEMIBOLD).size(metrics.font_heading).color(palette.text_primary),
                            text(format!("{} • {}", item.channel_title, item.kind))
                                .size(metrics.font_caption)
                                .color(palette.accent),
                            text(&item.description).size(metrics.font_caption).color(palette.text_secondary),
                        ]
                        .spacing(4)
                        .width(Length::Fill),
                    ]
                    .spacing(14)
                    .align_y(Alignment::Center),
                )
                .on_press(msg)
                .width(Length::Fill)
                .padding(12)
                .style(ghost_button_style(palette, metrics.radius_sm, false)),
            )
            .width(Length::Fill)
            .style(move |_| palette.card_container_style(metrics.radius_md));
            col = col.push(card);
        }
    }

    col.into()
}

fn filter_chip<'a>(
    label: String,
    active: bool,
    msg: UiMessage,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    button(
        text(label)
            .font(if active { FONT_UI_SEMIBOLD } else { iced::Font::DEFAULT })
            .size(metrics.font_micro),
    )
    .on_press(msg)
    .padding([5, 12])
    .style(chip_button_style(palette, active))
    .into()
}

fn render_watch_view<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let Some(ref video) = state.current_video else {
        return empty_state(
            "No video selected",
            loc.t("search.placeholder"),
            palette,
            metrics,
        );
    };

    let player_height = if state.fullscreen_mode {
        (state.window_height - 130.0).max(340.0)
    } else if state.theater_mode {
        480.0 * metrics.scale
    } else {
        380.0 * metrics.scale
    };

    let total_dur = state.player_duration_secs.max(video.duration.as_secs()).max(1);
    let pos_clamped = state.player_position_secs.min(total_dur);

    let player_surface = container(
        column![
            accent_badge(loc.t("player.embed_notice"), palette, metrics),
            Space::with_height(Length::Fixed(12.0)),
            text(&video.title)
                .font(FONT_UI_BOLD)
                .size(metrics.font_title)
                .color(Color::WHITE),
            text(format!(
                "{}  •  {} / {}",
                if state.player_playing { "▶ Playing" } else { "⏸ Paused" },
                format_seconds_hms(pos_clamped),
                format_seconds_hms(total_dur)
            ))
            .font(FONT_MONO_BOLD)
            .size(metrics.font_body)
            .color(Color::from_rgb8(210, 216, 228)),
        ]
        .spacing(6)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fixed(player_height))
    .padding(24)
    .style(move |_| container::Style {
        background: Some(Background::Color(Color::from_rgb8(9, 10, 14))),
        border: Border {
            color: palette.border_subtle,
            width: 1.0,
            radius: metrics.radius_lg.into(),
        },
        ..Default::default()
    });

    let seek_bar = row![
        text(format_seconds_hms(pos_clamped))
            .font(FONT_MONO_BOLD)
            .size(metrics.font_caption)
            .color(palette.text_secondary),
        container(
            slider(0_u32..=total_dur as u32, pos_clamped as u32, |v| {
                UiMessage::PlayerSeekTo(v as u64)
            })
        )
        .width(Length::Fill),
        text(format_seconds_hms(total_dur))
            .font(FONT_MONO_BOLD)
            .size(metrics.font_caption)
            .color(palette.text_secondary),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let transport_bar = row![
        button(
            text(if state.player_playing {
                loc.t("player.pause")
            } else {
                loc.t("player.play")
            })
            .font(FONT_UI_SEMIBOLD)
            .size(metrics.font_caption)
        )
        .on_press(UiMessage::PlayerTogglePlay)
        .padding([8, 18])
        .style(primary_button_style(palette, 999.0)),
        button(text("-10s").size(metrics.font_caption))
            .on_press(UiMessage::PlayerSeekRelative(-10))
            .padding([8, 12])
            .style(secondary_button_style(palette, 999.0)),
        button(text("+10s").size(metrics.font_caption))
            .on_press(UiMessage::PlayerSeekRelative(10))
            .padding([8, 12])
            .style(secondary_button_style(palette, 999.0)),
        button(
            text(if state.player_muted {
                loc.t("player.unmute")
            } else {
                loc.t("player.mute")
            })
            .size(metrics.font_caption)
        )
        .on_press(UiMessage::PlayerToggleMute)
        .padding([8, 12])
        .style(secondary_button_style(palette, 999.0)),
        container(
            slider(
                0.0_f32..=1.0_f32,
                state.player_volume,
                UiMessage::PlayerVolumeChanged,
            )
            .step(0.05_f32)
        )
        .width(Length::Fixed(96.0)),
        Space::with_width(Length::Fill),
        filter_chip("0.75x".into(), (state.player_rate - 0.75).abs() < 0.05, UiMessage::PlayerRateChanged(0.75), palette, metrics),
        filter_chip("1.0x".into(), (state.player_rate - 1.0).abs() < 0.05, UiMessage::PlayerRateChanged(1.0), palette, metrics),
        filter_chip("1.25x".into(), (state.player_rate - 1.25).abs() < 0.05, UiMessage::PlayerRateChanged(1.25), palette, metrics),
        filter_chip("1.5x".into(), (state.player_rate - 1.5).abs() < 0.05, UiMessage::PlayerRateChanged(1.5), palette, metrics),
        button(text(loc.t("player.theater_mode")).size(metrics.font_caption))
            .on_press(UiMessage::PlayerToggleTheater)
            .padding([7, 12])
            .style(secondary_button_style(palette, metrics.radius_sm)),
        button(text(loc.t("player.fullscreen")).size(metrics.font_caption))
            .on_press(UiMessage::PlayerToggleFullscreen)
            .padding([7, 12])
            .style(secondary_button_style(palette, metrics.radius_sm)),
        button(text(loc.t("player.mini_player")).size(metrics.font_caption))
            .on_press(UiMessage::PlayerToggleMini)
            .padding([7, 12])
            .style(secondary_button_style(palette, metrics.radius_sm)),
        button(text(loc.t("player.open_in_browser")).size(metrics.font_caption))
            .on_press(UiMessage::OpenInExternalBrowser(video.watch_url()))
            .padding([7, 12])
            .style(secondary_button_style(palette, metrics.radius_sm)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let is_sub = state.subscribed_ids.contains(&video.channel_id);
    let ch_tint = deterministic_tint(&video.channel_title);
    let action_bar = row![
        button(
            row![
                avatar_circle(&video.channel_title, ch_tint, 38.0),
                column![
                    text(&video.channel_title).font(FONT_UI_BOLD).size(metrics.font_heading),
                    text(format!("{} views • {}", video.formatted_view_count(), video.published_at.format("%Y-%m-%d")))
                        .size(metrics.font_caption)
                        .color(palette.text_secondary),
                ]
                .spacing(2),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .on_press(UiMessage::OpenChannel(video.channel_id.clone()))
        .padding([4, 8])
        .style(ghost_button_style(palette, metrics.radius_sm, false)),
        button(
            text(if is_sub {
                loc.t("watch.subscribed")
            } else {
                loc.t("watch.subscribe")
            })
            .font(FONT_UI_SEMIBOLD)
            .size(metrics.font_caption)
        )
        .on_press(UiMessage::ToggleChannelSubscribe(video.channel_id.clone()))
        .padding([8, 16])
        .style(move |t, s| {
            if is_sub {
                secondary_button_style(palette, 999.0)(t, s)
            } else {
                primary_button_style(palette, 999.0)(t, s)
            }
        }),
        Space::with_width(Length::Fill),
        filter_chip(
            format!("👍 {}", loc.t("watch.like")),
            state.current_video_rating == VideoRating::Like,
            UiMessage::RateCurrentVideo(VideoRating::Like),
            palette,
            metrics,
        ),
        filter_chip(
            format!("👎 {}", loc.t("watch.dislike")),
            state.current_video_rating == VideoRating::Dislike,
            UiMessage::RateCurrentVideo(VideoRating::Dislike),
            palette,
            metrics,
        ),
        button(text(loc.t("watch.save_later")).size(metrics.font_caption))
            .on_press(UiMessage::ToggleWatchLater(video.id.clone()))
            .padding([6, 12])
            .style(secondary_button_style(palette, 999.0)),
        button(text(loc.t("watch.bookmark_ts")).size(metrics.font_caption))
            .on_press(UiMessage::AddTimestampBookmark)
            .padding([6, 12])
            .style(secondary_button_style(palette, 999.0)),
        button(text(loc.t("watch.copy_ts_link")).size(metrics.font_caption))
            .on_press(UiMessage::CopyTimestampUrl)
            .padding([6, 12])
            .style(secondary_button_style(palette, 999.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut left_col = Column::new()
        .spacing(metrics.space_md)
        .width(Length::FillPortion(2))
        .push(text(&video.title).font(FONT_UI_BOLD).size(metrics.font_title))
        .push(action_bar)
        .push(
            container(
                column![
                    text(loc.t("watch.description"))
                        .font(FONT_UI_SEMIBOLD)
                        .size(metrics.font_heading),
                    text(&video.description)
                        .size(metrics.font_body)
                        .color(palette.text_secondary),
                ]
                .spacing(6),
            )
            .padding(14)
            .width(Length::Fill)
            .style(move |_| palette.card_container_style(metrics.radius_md)),
        );

    if !state.chapters.is_empty() {
        let mut ch_row = Row::new().spacing(6);
        for (ts, label) in &state.chapters {
            ch_row = ch_row.push(
                button(text(format!("{}  {}", format_seconds_hms(*ts), label)).size(metrics.font_micro))
                    .on_press(UiMessage::OpenVideoAt(video.id.clone(), *ts))
                    .padding([5, 12])
                    .style(secondary_button_style(palette, 999.0)),
            );
        }
        left_col = left_col
            .push(text(loc.t("watch.chapters")).font(FONT_UI_SEMIBOLD).size(metrics.font_heading))
            .push(ch_row);
    }

    // Local notes
    left_col = left_col.push(
        container(
            column![
                text(loc.t("watch.notes")).font(FONT_UI_SEMIBOLD).size(metrics.font_heading),
                row![
                    text_input(&loc.t("watch.note_placeholder"), &state.note_draft)
                        .on_input(UiMessage::NoteDraftChanged)
                        .padding([7, 12])
                        .style(input_style(palette, metrics.radius_sm))
                        .width(Length::Fill),
                    button(text(loc.t("settings.save")).size(metrics.font_caption))
                        .on_press(UiMessage::SaveVideoNote)
                        .padding([7, 14])
                        .style(secondary_button_style(palette, metrics.radius_sm)),
                ]
                .spacing(8),
            ]
            .spacing(8),
        )
        .padding(14)
        .width(Length::Fill)
        .style(move |_| palette.card_container_style(metrics.radius_md)),
    );

    // Collapsible Comments
    left_col = left_col.push(
        row![
            text(loc.t("watch.comments"))
                .font(FONT_UI_SEMIBOLD)
                .size(metrics.font_heading)
                .width(Length::Fill),
            button(text(loc.t("watch.toggle_comments")).size(metrics.font_caption))
                .on_press(UiMessage::ToggleCommentsPanel)
                .padding([5, 12])
                .style(secondary_button_style(palette, 999.0)),
        ]
        .align_y(Alignment::Center),
    );

    if state.comments_visible {
        left_col = left_col.push(
            row![
                text_input(&loc.t("watch.comment_placeholder"), &state.comment_draft)
                    .on_input(UiMessage::CommentDraftChanged)
                    .on_submit(UiMessage::SubmitComment)
                    .padding([8, 12])
                    .style(input_style(palette, metrics.radius_sm))
                    .width(Length::Fill),
                button(text(loc.t("watch.post_comment")).size(metrics.font_caption))
                    .on_press(UiMessage::SubmitComment)
                    .padding([8, 16])
                    .style(primary_button_style(palette, metrics.radius_sm)),
            ]
            .spacing(8),
        );

        for c in &state.comments {
            let ctint = deterministic_tint(&c.author_name);
            left_col = left_col.push(
                container(
                    row![
                        avatar_circle(&c.author_name, ctint, 30.0),
                        column![
                            text(format!("{}  •  👍 {}", c.author_name, c.like_count))
                                .font(FONT_UI_SEMIBOLD)
                                .size(metrics.font_caption)
                                .color(palette.accent),
                            text(&c.text).size(metrics.font_body),
                        ]
                        .spacing(3)
                        .width(Length::Fill),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Start),
                )
                .padding(12)
                .width(Length::Fill)
                .style(move |_| palette.card_container_style(metrics.radius_sm)),
            );
        }
    } else {
        left_col = left_col.push(pill_badge(loc.t("watch.comments_hidden"), palette, metrics));
    }

    // Queue & related right sidebar
    let mut right_col = Column::new()
        .spacing(metrics.space_sm)
        .width(Length::FillPortion(1))
        .push(
            row![
                text(loc.t("watch.queue"))
                    .font(FONT_UI_SEMIBOLD)
                    .size(metrics.font_heading)
                    .width(Length::Fill),
                button(text("Next ▶").size(metrics.font_micro))
                    .on_press(UiMessage::PlayNextInQueue)
                    .padding([4, 10])
                    .style(secondary_button_style(palette, 999.0)),
            ]
            .align_y(Alignment::Center),
        );

    for qv in state.queue.iter().chain(state.home_videos.iter()).take(6) {
        let qtint = deterministic_tint(&qv.channel_title);
        right_col = right_col.push(
            button(
                row![
                    avatar_circle(&qv.channel_title, qtint, 28.0),
                    column![
                        text(&qv.title).font(FONT_UI_SEMIBOLD).size(metrics.font_caption),
                        text(format!("{} • {}", qv.channel_title, qv.formatted_duration()))
                            .size(metrics.font_micro)
                            .color(palette.text_muted),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .on_press(UiMessage::OpenVideo(qv.id.clone()))
            .width(Length::Fill)
            .padding(8)
            .style(secondary_button_style(palette, metrics.radius_sm)),
        );
    }

    let details_layout: Element<'a, UiMessage> =
        if state.theater_mode || state.content_width() < 920.0 {
            column![left_col, right_col].spacing(metrics.space_lg).into()
        } else {
            row![left_col, right_col].spacing(metrics.space_lg).into()
        };

    column![player_surface, seek_bar, transport_bar, details_layout]
        .spacing(metrics.space_md)
        .into()
}

fn render_mini_player_bar<'a>(
    state: &'a UiState,
    video: &'a Video,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    container(
        row![
            text(format!("▶ {} — {}", video.title, video.channel_title))
                .font(FONT_UI_SEMIBOLD)
                .size(metrics.font_caption)
                .width(Length::Fill),
            button(
                text(if state.player_playing {
                    loc.t("player.pause")
                } else {
                    loc.t("player.play")
                })
                .size(metrics.font_micro)
            )
            .on_press(UiMessage::PlayerTogglePlay)
            .padding([5, 12])
            .style(primary_button_style(palette, 999.0)),
            button(text("Expand").size(metrics.font_micro))
                .on_press(UiMessage::OpenVideo(video.id.clone()))
                .padding([5, 12])
                .style(secondary_button_style(palette, 999.0)),
            button(text("✕").size(metrics.font_micro))
                .on_press(UiMessage::PlayerToggleMini)
                .padding([5, 8])
                .style(ghost_button_style(palette, metrics.radius_sm, false)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([8, 16])
    .width(Length::Fill)
    .style(move |_| panel_style(palette, palette.bg_elevated, 0.0))
    .into()
}

fn render_channel_view<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let Some(ref ch) = state.active_channel else {
        return empty_state("Channel not loaded", String::new(), palette, metrics);
    };

    let is_sub = state.subscribed_ids.contains(&ch.id);
    let is_fav = state.favorite_channels.contains(&ch.id);
    let tint = deterministic_tint(&ch.title);

    let header = container(
        row![
            avatar_circle(&ch.title, tint, 58.0),
            column![
                text(&ch.title).font(FONT_UI_BOLD).size(metrics.font_title),
                text(loc.t_with_args(
                    "channel.subscribers",
                    &[("count", &ch.formatted_subscriber_count())],
                ))
                .size(metrics.font_caption)
                .color(palette.accent),
                text(&ch.description)
                    .size(metrics.font_body)
                    .color(palette.text_secondary),
            ]
            .spacing(4)
            .width(Length::Fill),
            button(
                text(if is_sub {
                    loc.t("watch.subscribed")
                } else {
                    loc.t("watch.subscribe")
                })
                .font(FONT_UI_SEMIBOLD)
                .size(metrics.font_caption)
            )
            .on_press(UiMessage::ToggleChannelSubscribe(ch.id.clone()))
            .padding([8, 16])
            .style(primary_button_style(palette, 999.0)),
            button(
                text(if is_fav {
                    loc.t("channel.favorite_remove")
                } else {
                    loc.t("channel.favorite_add")
                })
                .size(metrics.font_caption)
            )
            .on_press(UiMessage::ToggleChannelFavorite(ch.id.clone()))
            .padding([8, 14])
            .style(secondary_button_style(palette, 999.0)),
        ]
        .spacing(14)
        .align_y(Alignment::Center),
    )
    .padding(18)
    .width(Length::Fill)
    .style(move |_| palette.card_container_style(metrics.radius_md));

    let tabs = row![
        filter_chip(loc.t("channel.videos"), state.channel_tab == ChannelTab::Videos, UiMessage::SwitchChannelTab(ChannelTab::Videos), palette, metrics),
        filter_chip(loc.t("channel.playlists"), state.channel_tab == ChannelTab::Playlists, UiMessage::SwitchChannelTab(ChannelTab::Playlists), palette, metrics),
        filter_chip(loc.t("channel.live"), state.channel_tab == ChannelTab::Live, UiMessage::SwitchChannelTab(ChannelTab::Live), palette, metrics),
        Space::with_width(Length::Fill),
        text_input("Filter channel...", &state.channel_filter)
            .on_input(UiMessage::ChannelFilterChanged)
            .padding([6, 12])
            .width(Length::Fixed(220.0))
            .style(input_style(palette, 999.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut col = Column::new().spacing(metrics.space_md).push(header).push(tabs);

    match state.channel_tab {
        ChannelTab::Videos | ChannelTab::Live => {
            let needle = state.channel_filter.to_lowercase();
            let matching: Vec<&Video> = state
                .channel_videos
                .iter()
                .filter(|v| {
                    let tab_ok = state.channel_tab != ChannelTab::Live || v.is_live();
                    let filter_ok = needle.is_empty() || v.title.to_lowercase().contains(&needle);
                    tab_ok && filter_ok
                })
                .collect();
            if matching.is_empty() {
                col = col.push(empty_state(loc.t("search.empty"), String::new(), palette, metrics));
            } else {
                let cols = metrics.grid_columns(state.content_width());
                let mut grid2 = Column::new().spacing(metrics.space_md);
                for chunk in matching.chunks(cols) {
                    let mut r = Row::new().spacing(metrics.space_md);
                    for v in chunk {
                        r = r.push(container(render_video_card(v, state, palette, metrics)).width(Length::FillPortion(1)));
                    }
                    for _ in chunk.len()..cols {
                        r = r.push(Space::with_width(Length::FillPortion(1)));
                    }
                    grid2 = grid2.push(r);
                }
                col = col.push(grid2);
            }
        }
        ChannelTab::Playlists => {
            for pl in &state.channel_playlists {
                col = col.push(
                    container(
                        column![
                            text(&pl.title).font(FONT_UI_SEMIBOLD).size(metrics.font_heading),
                            text(format!("{} items • {}", pl.item_count, pl.description))
                                .size(metrics.font_caption)
                                .color(palette.text_secondary),
                        ]
                        .spacing(4),
                    )
                    .padding(14)
                    .width(Length::Fill)
                    .style(move |_| palette.card_container_style(metrics.radius_sm)),
                );
            }
        }
    }

    col.into()
}

fn render_subscriptions_view<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let mut col = Column::new()
        .spacing(metrics.space_md)
        .push(text(loc.t("nav.subscriptions")).font(FONT_UI_BOLD).size(metrics.font_title))
        .push(pill_badge(loc.t("library.device_sync_note"), palette, metrics));

    if state.subscriptions.is_empty() {
        col = col.push(empty_state(
            loc.t("search.empty"),
            loc.t("auth.oauth_note"),
            palette,
            metrics,
        ));
    } else {
        for ch in &state.subscriptions {
            let tint = deterministic_tint(&ch.title);
            col = col.push(
                container(
                    row![
                        avatar_circle(&ch.title, tint, 42.0),
                        column![
                            text(&ch.title).font(FONT_UI_SEMIBOLD).size(metrics.font_heading),
                            text(&ch.description).size(metrics.font_caption).color(palette.text_secondary),
                        ]
                        .spacing(3)
                        .width(Length::Fill),
                        button(text(loc.t("channel.videos")).size(metrics.font_caption))
                            .on_press(UiMessage::OpenChannel(ch.id.clone()))
                            .padding([7, 14])
                            .style(secondary_button_style(palette, 999.0)),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                )
                .padding(14)
                .width(Length::Fill)
                .style(move |_| palette.card_container_style(metrics.radius_md)),
            );
        }
    }
    col.into()
}

fn render_shorts_view<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let controls = row![
        text(loc.t("nav.shorts")).font(FONT_UI_BOLD).size(metrics.font_title).width(Length::Fill),
        checkbox(loc.t("shorts.autoplay"), state.shorts_autoplay)
            .on_toggle(UiMessage::ToggleShortsAutoplay),
        checkbox(loc.t("shorts.hide_section"), state.hide_shorts)
            .on_toggle(UiMessage::ToggleHideShorts),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let mut col = Column::new()
        .spacing(metrics.space_md)
        .push(controls)
        .push(pill_badge(loc.t("shorts.api_note"), palette, metrics));

    if let Some(short) = state.shorts_videos.get(state.shorts_index) {
        let card = container(
            column![
                accent_badge("⚡ SHORT", palette, metrics),
                text(&short.title).font(FONT_UI_BOLD).size(metrics.font_title),
                text(format!("{} • {}", short.channel_title, short.formatted_duration()))
                    .size(metrics.font_body)
                    .color(palette.text_secondary),
                Space::with_height(Length::Fixed(16.0)),
                row![
                    button(text(loc.t("shorts.prev")).size(metrics.font_caption))
                        .on_press(UiMessage::ShortsPrev)
                        .padding([8, 14])
                        .style(secondary_button_style(palette, 999.0)),
                    button(text(loc.t("player.play")).font(FONT_UI_SEMIBOLD).size(metrics.font_caption))
                        .on_press(UiMessage::OpenVideo(short.id.clone()))
                        .padding([8, 20])
                        .style(primary_button_style(palette, 999.0)),
                    button(text(loc.t("shorts.next")).size(metrics.font_caption))
                        .on_press(UiMessage::ShortsNext)
                        .padding([8, 14])
                        .style(secondary_button_style(palette, 999.0)),
                ]
                .spacing(10),
            ]
            .spacing(10)
            .align_x(Alignment::Center),
        )
        .width(Length::Fixed(380.0))
        .center_x(Length::Fixed(380.0))
        .center_y(Length::Fixed(500.0))
        .padding(24)
        .style(move |_| palette.card_container_style(metrics.radius_lg));

        col = col.push(container(card).width(Length::Fill).center_x(Length::Fill));
    } else {
        col = col.push(empty_state(loc.t("search.empty"), String::new(), palette, metrics));
    }

    col.into()
}

fn render_live_view<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let header = row![
        text(loc.t("nav.live")).font(FONT_UI_BOLD).size(metrics.font_title).width(Length::Fill),
        button(text(loc.t("live.toggle_chat")).size(metrics.font_caption))
            .on_press(UiMessage::ToggleLiveChat)
            .padding([7, 14])
            .style(secondary_button_style(palette, 999.0)),
    ]
    .align_y(Alignment::Center);

    let mut col = Column::new().spacing(metrics.space_md).push(header);
    if state.live_videos.is_empty() {
        col = col.push(empty_state(loc.t("search.empty"), String::new(), palette, metrics));
    } else {
        col = col.push(render_video_grid(&state.live_videos, state, palette, metrics));
    }
    col.into()
}

fn render_history_view<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let header = row![
        text(loc.t("history.title")).font(FONT_UI_BOLD).size(metrics.font_title).width(Length::Fill),
        pill_badge(loc.t("home.local_badge"), palette, metrics),
        button(text(loc.t("history.clear")).size(metrics.font_caption))
            .on_press(UiMessage::ClearWatchHistory)
            .padding([7, 14])
            .style(secondary_button_style(palette, 999.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut col = Column::new().spacing(metrics.space_md).push(header);
    if state.history.is_empty() {
        col = col.push(empty_state(
            loc.t("history.empty"),
            loc.t("library.device_sync_note"),
            palette,
            metrics,
        ));
    } else {
        for item in &state.history {
            col = col.push(
                button(
                    row![
                        text(&item.title).font(FONT_UI_SEMIBOLD).size(metrics.font_body).width(Length::Fill),
                        text(format!(
                            "{} • {}",
                            item.channel_title,
                            format_seconds_hms(item.playback_position_seconds)
                        ))
                        .size(metrics.font_caption)
                        .color(palette.text_secondary),
                    ]
                    .align_y(Alignment::Center),
                )
                .on_press(UiMessage::OpenVideoAt(
                    item.video_id.clone(),
                    item.playback_position_seconds,
                ))
                .width(Length::Fill)
                .padding(12)
                .style(secondary_button_style(palette, metrics.radius_sm)),
            );
        }
    }
    col.into()
}

fn render_playlists_view<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let mut col = Column::new()
        .spacing(metrics.space_md)
        .push(text(loc.t("nav.playlists")).font(FONT_UI_BOLD).size(metrics.font_title))
        .push(pill_badge(loc.t("library.device_sync_note"), palette, metrics))
        .push(
            row![
                text_input(&loc.t("library.create_playlist"), &state.new_playlist_draft)
                    .on_input(UiMessage::CreateLocalPlaylistDraftChanged)
                    .on_submit(UiMessage::CreateLocalPlaylist)
                    .padding([8, 12])
                    .style(input_style(palette, metrics.radius_sm))
                    .width(Length::Fill),
                button(text(loc.t("library.create_playlist")).font(FONT_UI_SEMIBOLD).size(metrics.font_caption))
                    .on_press(UiMessage::CreateLocalPlaylist)
                    .padding([8, 14])
                    .style(primary_button_style(palette, metrics.radius_sm)),
                button(text(loc.t("library.export_playlists")).size(metrics.font_caption))
                    .on_press(UiMessage::ExportLocalPlaylists)
                    .padding([8, 14])
                    .style(secondary_button_style(palette, metrics.radius_sm)),
            ]
            .spacing(8),
        );

    if state.playlists.is_empty() {
        col = col.push(empty_state(
            loc.t("library.empty_playlists"),
            String::new(),
            palette,
            metrics,
        ));
    } else {
        for pl in &state.playlists {
            let sync_tag = if pl.is_local {
                loc.t("home.local_badge")
            } else {
                "YouTube".to_string()
            };
            col = col.push(
                container(
                    row![
                        column![
                            text(&pl.title).font(FONT_UI_SEMIBOLD).size(metrics.font_heading),
                            text(format!("{} items • {}", pl.item_count, pl.description))
                                .size(metrics.font_caption)
                                .color(palette.text_secondary),
                        ]
                        .spacing(3)
                        .width(Length::Fill),
                        pill_badge(sync_tag, palette, metrics),
                    ]
                    .align_y(Alignment::Center),
                )
                .padding(14)
                .width(Length::Fill)
                .style(move |_| palette.card_container_style(metrics.radius_md)),
            );
        }
    }

    if !state.bookmarks.is_empty() {
        col = col.push(text(loc.t("library.bookmarks")).font(FONT_UI_BOLD).size(metrics.font_heading));
        for b in &state.bookmarks {
            col = col.push(
                button(text(format!(
                    "[{}] {} ({})",
                    b.formatted_timestamp(),
                    b.title,
                    b.video_id
                ))
                .size(metrics.font_caption))
                .on_press(UiMessage::OpenVideoAt(
                    b.video_id.clone(),
                    b.timestamp_seconds,
                ))
                .width(Length::Fill)
                .padding(10)
                .style(secondary_button_style(palette, metrics.radius_sm)),
            );
        }
    }

    col.into()
}

fn render_favorites_view<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let mut col = Column::new()
        .spacing(metrics.space_md)
        .push(text(loc.t("nav.favorites")).font(FONT_UI_BOLD).size(metrics.font_title));
    if state.favorite_channels.is_empty() {
        col = col.push(empty_state(loc.t("search.empty"), String::new(), palette, metrics));
    } else {
        for cid in &state.favorite_channels {
            col = col.push(
                button(text(format!("★  {}", cid)).size(metrics.font_body))
                    .on_press(UiMessage::OpenChannel(cid.clone()))
                    .padding(12)
                    .width(Length::Fill)
                    .style(secondary_button_style(palette, metrics.radius_sm)),
            );
        }
    }
    col.into()
}

fn render_settings_view<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let tabs = row![
        filter_chip(loc.t("settings.performance"), state.settings_tab == SettingsTab::Performance, UiMessage::SwitchSettingsTab(SettingsTab::Performance), palette, metrics),
        filter_chip(loc.t("settings.appearance"), state.settings_tab == SettingsTab::Appearance, UiMessage::SwitchSettingsTab(SettingsTab::Appearance), palette, metrics),
        filter_chip(loc.t("settings.general"), state.settings_tab == SettingsTab::General, UiMessage::SwitchSettingsTab(SettingsTab::General), palette, metrics),
        filter_chip(loc.t("settings.privacy"), state.settings_tab == SettingsTab::Privacy, UiMessage::SwitchSettingsTab(SettingsTab::Privacy), palette, metrics),
        filter_chip(loc.t("settings.google_api"), state.settings_tab == SettingsTab::GoogleApi, UiMessage::SwitchSettingsTab(SettingsTab::GoogleApi), palette, metrics),
        filter_chip(loc.t("settings.updates"), state.settings_tab == SettingsTab::Updates, UiMessage::SwitchSettingsTab(SettingsTab::Updates), palette, metrics),
        Space::with_width(Length::Fill),
        button(text(loc.t("settings.save")).font(FONT_UI_SEMIBOLD).size(metrics.font_caption))
            .on_press(UiMessage::SaveSettings)
            .padding([7, 16])
            .style(primary_button_style(palette, 999.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut col = Column::new()
        .spacing(metrics.space_md)
        .push(text(loc.t("settings.title")).font(FONT_UI_BOLD).size(metrics.font_title))
        .push(tabs);

    match state.settings_tab {
        SettingsTab::Performance => {
            let p = state.settings.performance.profile;
            let profiles = row![
                filter_chip(loc.t("settings.profile.ultralite"), p == PerformanceProfileType::UltraLite, UiMessage::SelectPerformanceProfile(PerformanceProfileType::UltraLite), palette, metrics),
                filter_chip(loc.t("settings.profile.balanced"), p == PerformanceProfileType::Balanced, UiMessage::SelectPerformanceProfile(PerformanceProfileType::Balanced), palette, metrics),
                filter_chip(loc.t("settings.profile.high_quality"), p == PerformanceProfileType::HighQuality, UiMessage::SelectPerformanceProfile(PerformanceProfileType::HighQuality), palette, metrics),
                filter_chip(loc.t("settings.profile.custom"), p == PerformanceProfileType::Custom, UiMessage::SelectPerformanceProfile(PerformanceProfileType::Custom), palette, metrics),
                Space::with_width(Length::Fill),
                button(text(loc.t("settings.recommend_profile")).size(metrics.font_caption))
                    .on_press(UiMessage::RecommendProfileForHardware)
                    .padding([7, 14])
                    .style(secondary_button_style(palette, 999.0)),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            let perf_stats = row![
                stat_tile("Max Quality", state.settings.performance.max_resolution.clone(), palette, metrics),
                stat_tile("Thumbnail Cache Limit", format!("{} MB", state.settings.performance.cache_size_mb), palette, metrics),
                stat_tile("Parallel Requests", state.settings.performance.max_parallel_requests.to_string(), palette, metrics),
            ]
            .spacing(metrics.space_md);

            col = col
                .push(profiles)
                .push(perf_stats)
                .push(
                    container(
                        column![
                            checkbox(loc.t("settings.h264ify"), state.settings.h264ify_enabled)
                                .on_toggle(UiMessage::ToggleH264ify),
                            checkbox(loc.t("settings.animations"), state.settings.performance.enable_animations)
                                .on_toggle(UiMessage::ToggleAnimations),
                            checkbox(loc.t("settings.comments_default"), state.settings.performance.enable_comments)
                                .on_toggle(UiMessage::ToggleAutoComments),
                        ]
                        .spacing(12),
                    )
                    .padding(16)
                    .width(Length::Fill)
                    .style(move |_| palette.card_container_style(metrics.radius_md)),
                );
        }
        SettingsTab::Appearance => {
            let current_theme = ThemeKind::from_key(&state.settings.theme);
            let theme_row = row![
                text(loc.t("settings.theme")).font(FONT_UI_SEMIBOLD).size(metrics.font_body),
                filter_chip("Graphite Dark".into(), current_theme == ThemeKind::GraphiteDark, UiMessage::SelectTheme(ThemeKind::GraphiteDark), palette, metrics),
                filter_chip("Pure Black".into(), current_theme == ThemeKind::PureBlack, UiMessage::SelectTheme(ThemeKind::PureBlack), palette, metrics),
                filter_chip("Arctic Light".into(), current_theme == ThemeKind::ArcticLight, UiMessage::SelectTheme(ThemeKind::ArcticLight), palette, metrics),
                filter_chip("System".into(), current_theme == ThemeKind::System, UiMessage::SelectTheme(ThemeKind::System), palette, metrics),
                filter_chip("Custom".into(), current_theme == ThemeKind::Custom, UiMessage::SelectTheme(ThemeKind::Custom), palette, metrics),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            let current_density = DensityMode::from_key(&state.settings.density);
            let density_row = row![
                text(loc.t("settings.density")).font(FONT_UI_SEMIBOLD).size(metrics.font_body),
                filter_chip("Compact".into(), current_density == DensityMode::Compact, UiMessage::SelectDensity(DensityMode::Compact), palette, metrics),
                filter_chip("Standard".into(), current_density == DensityMode::Standard, UiMessage::SelectDensity(DensityMode::Standard), palette, metrics),
                filter_chip("Comfortable".into(), current_density == DensityMode::Comfortable, UiMessage::SelectDensity(DensityMode::Comfortable), palette, metrics),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            let accent_row = row![
                text(loc.t("settings.accent")).font(FONT_UI_SEMIBOLD).size(metrics.font_body),
                text_input("#FF2D55", &state.settings.accent_color)
                    .on_input(UiMessage::AccentColorChanged)
                    .padding([6, 12])
                    .width(Length::Fixed(130.0))
                    .style(input_style(palette, metrics.radius_sm)),
                text(format!("{}: {:.2}x", loc.t("settings.scale"), state.settings.ui_scale)).size(metrics.font_body),
                container(slider(0.8_f32..=1.4_f32, state.settings.ui_scale, UiMessage::UiScaleChanged).step(0.05_f32))
                    .width(Length::Fixed(160.0)),
            ]
            .spacing(12)
            .align_y(Alignment::Center);

            col = col.push(
                container(column![theme_row, density_row, accent_row].spacing(14))
                    .padding(16)
                    .width(Length::Fill)
                    .style(move |_| palette.card_container_style(metrics.radius_md)),
            );
        }
        SettingsTab::General => {
            let lang = state.settings.language.as_str();
            let lang_row = row![
                text(loc.t("settings.language")).font(FONT_UI_SEMIBOLD).size(metrics.font_body),
                filter_chip("English".into(), lang == "en", UiMessage::SelectLocale("en".into()), palette, metrics),
                filter_chip("Українська".into(), lang == "uk", UiMessage::SelectLocale("uk".into()), palette, metrics),
                filter_chip("Русский".into(), lang == "ru", UiMessage::SelectLocale("ru".into()), palette, metrics),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            col = col.push(
                container(lang_row)
                    .padding(16)
                    .width(Length::Fill)
                    .style(move |_| palette.card_container_style(metrics.radius_md)),
            );
        }
        SettingsTab::Privacy => {
            col = col.push(
                container(
                    column![
                        checkbox(loc.t("settings.private_mode"), state.private_mode)
                            .on_toggle(UiMessage::TogglePrivateMode),
                        row![
                            button(text(loc.t("settings.clear_cache")).size(metrics.font_caption))
                                .on_press(UiMessage::ClearThumbnailCache)
                                .padding([8, 14])
                                .style(secondary_button_style(palette, metrics.radius_sm)),
                            button(text(loc.t("history.clear")).size(metrics.font_caption))
                                .on_press(UiMessage::ClearWatchHistory)
                                .padding([8, 14])
                                .style(secondary_button_style(palette, metrics.radius_sm)),
                        ]
                        .spacing(10),
                    ]
                    .spacing(14),
                )
                .padding(16)
                .width(Length::Fill)
                .style(move |_| palette.card_container_style(metrics.radius_md)),
            );
        }
        SettingsTab::GoogleApi => {
            col = col.push(render_google_api_settings(state, loc, palette, metrics));
        }
        SettingsTab::Updates => {
            col = col.push(
                container(
                    row![
                        button(text(loc.t("settings.check_updates")).font(FONT_UI_SEMIBOLD).size(metrics.font_caption))
                            .on_press(UiMessage::CheckForUpdates)
                            .padding([8, 16])
                            .style(primary_button_style(palette, 999.0)),
                        text(
                            state
                                .update_status_text
                                .clone()
                                .unwrap_or_else(|| format!("v{}", env!("CARGO_PKG_VERSION"))),
                        )
                        .size(metrics.font_body)
                        .color(palette.text_secondary),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                )
                .padding(16)
                .width(Length::Fill)
                .style(move |_| palette.card_container_style(metrics.radius_md)),
            );
        }
    }

    col.into()
}

fn render_diagnostics_view<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let header = row![
        text(loc.t("diagnostics.title")).font(FONT_UI_BOLD).size(metrics.font_title).width(Length::Fill),
        button(text(loc.t("home.refresh")).size(metrics.font_caption))
            .on_press(UiMessage::RefreshDiagnostics)
            .padding([7, 14])
            .style(secondary_button_style(palette, 999.0)),
        button(text(loc.t("diagnostics.export_report")).font(FONT_UI_SEMIBOLD).size(metrics.font_caption))
            .on_press(UiMessage::ExportDiagnosticReport)
            .padding([7, 16])
            .style(primary_button_style(palette, 999.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut col = Column::new().spacing(metrics.space_md).push(header);

    let rss_str = state
        .diagnostics
        .as_ref()
        .map(|d| format!("{} MB", d.metrics.process_memory_mb))
        .unwrap_or_else(|| "Measuring...".into());

    let cpu_str = state
        .diagnostics
        .as_ref()
        .map(|d| format!("{:.1}%", d.metrics.cpu_usage_percent))
        .unwrap_or_else(|| "0.0%".into());

    let row1 = row![
        stat_tile("OS / Arch", format!("{} {} ({})", state.hardware.os_name, state.hardware.os_version, state.hardware.cpu_arch), palette, metrics),
        stat_tile("FluxTube RSS", rss_str, palette, metrics),
        stat_tile("CPU Usage", cpu_str, palette, metrics),
    ]
    .spacing(metrics.space_md);

    let decoders_txt = state
        .hardware
        .available_decoders
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let quota_txt = state
        .quota_general
        .as_ref()
        .map(|q| format!("{} / {}", q.used_today, q.daily_limit))
        .unwrap_or_else(|| "0 / 10000".into());

    let row2 = row![
        stat_tile(loc.t("diagnostics.graphics_backend"), state.hardware.graphics_backend.clone(), palette, metrics),
        stat_tile(loc.t("diagnostics.video_backend"), "Isolated WebView (wry + h264ify)".to_string(), palette, metrics),
        stat_tile("API Quota", quota_txt, palette, metrics),
    ]
    .spacing(metrics.space_md);

    let row3 = row![
        stat_tile(loc.t("diagnostics.decoders"), decoders_txt, palette, metrics),
        stat_tile("Cache Size", format!("{} MB", state.cache_size_mb), palette, metrics),
        stat_tile(
            "Auth",
            if state.active_account.is_some() {
                "Authenticated (Google OAuth)".to_string()
            } else {
                loc.t("auth.guest")
            },
            palette,
            metrics,
        ),
    ]
    .spacing(metrics.space_md);

    col = col.push(row1).push(row2).push(row3);
    col.into()
}

fn mask_display(val: &str) -> String {
    let t = val.trim();
    if t.is_empty() {
        return "Not configured".to_string();
    }
    let chars: Vec<char> = t.chars().collect();
    if chars.len() <= 12 {
        "Configured (hidden)".to_string()
    } else {
        let head: String = chars[..6].iter().collect();
        let tail: String = chars[chars.len() - 6..].iter().collect();
        format!("{}...{}", head, tail)
    }
}

fn render_google_api_settings<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let oauth_configured = !state.cred_client_id.trim().is_empty();
    let api_key_configured = !state.cred_api_key.trim().is_empty();
    let quota_text = state
        .quota_general
        .as_ref()
        .map(|q| format!("{} / {} units", q.used_today, q.daily_limit))
        .unwrap_or_else(|| "0 / 10000 units".to_string());

    let top_tiles = row![
        stat_tile(
            "OAuth 2.0 Config",
            if oauth_configured {
                mask_display(&state.cred_client_id)
            } else {
                "Not configured".to_string()
            },
            palette,
            metrics,
        ),
        stat_tile(
            loc.t("auth.session_status"),
            state.auth_session_status.clone(),
            palette,
            metrics,
        ),
        stat_tile(
            loc.t("auth.api_status"),
            state.api_connection_status.clone(),
            palette,
            metrics,
        ),
        stat_tile("Daily Quota", quota_text, palette, metrics),
    ]
    .spacing(metrics.space_md);

    let channel_summary = if let Some(ref ch) = state.active_user_channel {
        format!(
            "{}  ({})  •  {} subscribers  •  {} videos",
            ch.title,
            ch.custom_url.clone().unwrap_or_else(|| ch.id.clone()),
            ch.formatted_subscriber_count(),
            ch.video_count
        )
    } else if state.active_account.is_some() {
        "No linked YouTube channel fetched yet (or account has no YouTube channel)".to_string()
    } else {
        "Guest mode — sign in to inspect your personal YouTube channel".to_string()
    };

    let mut diag_col = column![
        text(loc.t("auth.your_channel"))
            .font(FONT_UI_SEMIBOLD)
            .size(metrics.font_body),
        text(channel_summary)
            .size(metrics.font_caption)
            .color(palette.text_secondary),
        text(format!(
            "{}: {}",
            loc.t("auth.last_refresh"),
            state
                .auth_last_refresh_at
                .clone()
                .unwrap_or_else(|| "Not refreshed in this session".to_string())
        ))
        .size(metrics.font_caption)
        .color(palette.text_secondary),
        text(format!(
            "Guest Public API Key: {}   |   Token Storage: {}",
            if api_key_configured {
                mask_display(&state.cred_api_key)
            } else {
                "Not set (Demo / OAuth-only)".to_string()
            },
            if cfg!(target_os = "macos") {
                "macOS Keychain (refresh token) + RAM (access token)"
            } else {
                "Windows Credential Manager (refresh token) + RAM (access token)"
            }
        ))
        .size(metrics.font_caption)
        .color(palette.text_secondary),
        text(format!(
            "Feature Availability: Public search/watch = {}  |  Personal feeds/comments/likes/playlists = {}",
            if api_key_configured || state.active_account.is_some() {
                "Available"
            } else {
                "Offline Demo"
            },
            if state.active_account.is_some() && !state.auth_reauth_required {
                "Authorized"
            } else {
                "Requires Google Sign-In"
            }
        ))
        .size(metrics.font_caption)
        .color(palette.text_secondary),
    ]
    .spacing(6);

    if !state.auth_granted_scopes.is_empty() {
        diag_col = diag_col.push(
            text(format!(
                "{}: {}",
                loc.t("auth.granted_scopes"),
                state.auth_granted_scopes.join(", ")
            ))
            .size(metrics.font_micro)
            .color(palette.text_muted),
        );
    }

    if let Some(ref api_err) = state.last_api_error {
        diag_col = diag_col.push(
            text(format!("Last API Error: {}", api_err))
                .size(metrics.font_caption)
                .color(palette.danger),
        );
    }

    if let Some(ref auth_err) = state.auth_last_error {
        diag_col = diag_col.push(
            text(format!("Last Auth Error: {}", auth_err))
                .size(metrics.font_caption)
                .color(palette.danger),
        );
    }

    let mut auth_actions = row![
        button(text(loc.t("auth.import_json")).font(FONT_UI_SEMIBOLD).size(metrics.font_caption))
            .on_press(UiMessage::ImportOAuthJson)
            .padding([8, 14])
            .style(primary_button_style(palette, metrics.radius_sm)),
        button(text(loc.t("auth.test_connection")).size(metrics.font_caption))
            .on_press(UiMessage::TestGoogleApiConnection)
            .padding([8, 14])
            .style(secondary_button_style(palette, metrics.radius_sm)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if state.pkce_auth_loading {
        auth_actions = auth_actions.push(
            button(text(loc.t("auth.cancel_login")).size(metrics.font_caption))
                .on_press(UiMessage::CancelGoogleSignIn)
                .padding([8, 14])
                .style(secondary_button_style(palette, metrics.radius_sm)),
        );
    } else if state.auth_reauth_required {
        auth_actions = auth_actions.push(
            button(text(loc.t("auth.retry_login")).font(FONT_UI_SEMIBOLD).size(metrics.font_caption))
                .on_press(UiMessage::StartGoogleSignIn)
                .padding([8, 14])
                .style(primary_button_style(palette, metrics.radius_sm)),
        );
    } else {
        auth_actions = auth_actions.push(
            button(text(loc.t("auth.login")).size(metrics.font_caption))
                .on_press(UiMessage::ToggleAccountModal(true))
                .padding([8, 14])
                .style(secondary_button_style(palette, metrics.radius_sm)),
        );
    }

    if let Some(ref acc) = state.active_account {
        auth_actions = auth_actions.push(
            button(text(loc.t("auth.revoke_access")).size(metrics.font_caption))
                .on_press(UiMessage::RevokeGoogleAccess(acc.id.clone()))
                .padding([8, 14])
                .style(secondary_button_style(palette, metrics.radius_sm)),
        );
    }

    let creds_editor = container(
        column![
            text(loc.t("auth.credentials_title"))
                .font(FONT_UI_SEMIBOLD)
                .size(metrics.font_body),
            text(format!(
                "Desktop OAuth Project: {}  |  Redirect policy: http://127.0.0.1:<random_port>/callback (PKCE S256)",
                state
                    .cred_project_id
                    .clone()
                    .unwrap_or_else(|| "Manual / .env / Imported JSON".to_string())
            ))
            .size(metrics.font_micro)
            .color(palette.text_muted),
            text_input("GOOGLE_CLIENT_ID (...apps.googleusercontent.com)", &state.cred_client_id)
                .on_input(UiMessage::CredClientIdChanged)
                .padding([7, 10])
                .size(metrics.font_caption)
                .style(input_style(palette, metrics.radius_sm)),
            text_input("Optional Client Secret (hidden; Desktop clients use PKCE)", &state.cred_client_secret)
                .on_input(UiMessage::CredClientSecretChanged)
                .secure(true)
                .padding([7, 10])
                .size(metrics.font_caption)
                .style(input_style(palette, metrics.radius_sm)),
            text_input("YOUTUBE_API_KEY (for Guest public data)", &state.cred_api_key)
                .on_input(UiMessage::CredApiKeyChanged)
                .padding([7, 10])
                .size(metrics.font_caption)
                .style(input_style(palette, metrics.radius_sm)),
            row![
                button(text(loc.t("auth.save_credentials")).font(FONT_UI_SEMIBOLD).size(metrics.font_caption))
                    .on_press(UiMessage::SaveGoogleCredentials)
                    .padding([7, 14])
                    .style(primary_button_style(palette, metrics.radius_sm)),
                button(text(loc.t("auth.clear_credentials")).size(metrics.font_caption))
                    .on_press(UiMessage::ClearGoogleCredentials)
                    .padding([7, 14])
                    .style(secondary_button_style(palette, metrics.radius_sm)),
            ]
            .spacing(8),
        ]
        .spacing(8),
    )
    .padding(16)
    .width(Length::Fill)
    .style(move |_| palette.card_container_style(metrics.radius_md));

    column![
        top_tiles,
        container(column![diag_col, auth_actions].spacing(12))
            .padding(16)
            .width(Length::Fill)
            .style(move |_| palette.card_container_style(metrics.radius_md)),
        creds_editor,
    ]
    .spacing(metrics.space_md)
    .into()
}

fn render_account_modal<'a>(
    state: &'a UiState,
    loc: &'a LocalizationManager,
    palette: ThemePalette,
    metrics: LayoutMetrics,
) -> Element<'a, UiMessage> {
    let header = row![
        text(loc.t("auth.switch_account"))
            .font(FONT_UI_BOLD)
            .size(metrics.font_title)
            .width(Length::Fill),
        button(text(loc.t("auth.import_json")).size(metrics.font_micro))
            .on_press(UiMessage::ImportOAuthJson)
            .padding([5, 10])
            .style(secondary_button_style(palette, 999.0)),
        button(text("✕").size(metrics.font_body))
            .on_press(UiMessage::ToggleAccountModal(false))
            .padding([4, 10])
            .style(ghost_button_style(palette, metrics.radius_sm, false)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let auth_tabs = row![
        filter_chip(
            loc.t("auth.pkce_tab"),
            state.auth_tab == AuthTab::BrowserPkce,
            UiMessage::SwitchAuthTab(AuthTab::BrowserPkce),
            palette,
            metrics,
        ),
        filter_chip(
            loc.t("auth.device_tab"),
            state.auth_tab == AuthTab::DeviceCode,
            UiMessage::SwitchAuthTab(AuthTab::DeviceCode),
            palette,
            metrics,
        ),
        Space::with_width(Length::Fill),
        button(text(loc.t("auth.credentials_title")).size(metrics.font_micro))
            .on_press(UiMessage::ToggleCredentialsForm)
            .padding([5, 10])
            .style(secondary_button_style(palette, 999.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let auth_panel: Element<'a, UiMessage> = match state.auth_tab {
        AuthTab::BrowserPkce => {
            let mut pkce_col = column![
                text(loc.t("auth.oauth_note"))
                    .size(metrics.font_caption)
                    .color(palette.text_secondary),
            ]
            .spacing(10);

            if let Some(ref err_msg) = state.auth_last_error {
                pkce_col = pkce_col.push(
                    text(err_msg)
                        .size(metrics.font_caption)
                        .color(palette.danger),
                );
            }

            if state.pkce_auth_loading {
                pkce_col = pkce_col.push(
                    row![
                        text(loc.t("auth.waiting_approval"))
                            .font(FONT_UI_SEMIBOLD)
                            .size(metrics.font_caption)
                            .color(palette.accent)
                            .width(Length::Fill),
                        button(text(loc.t("auth.cancel_login")).size(metrics.font_caption))
                            .on_press(UiMessage::CancelGoogleSignIn)
                            .padding([8, 14])
                            .style(secondary_button_style(palette, metrics.radius_sm)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
            } else {
                let btn_label = if state.auth_reauth_required {
                    loc.t("auth.retry_login")
                } else {
                    format!("+ {}", loc.t("auth.login"))
                };
                pkce_col = pkce_col.push(
                    button(
                        text(btn_label)
                            .font(FONT_UI_SEMIBOLD)
                            .size(metrics.font_body),
                    )
                    .on_press(UiMessage::StartGoogleSignIn)
                    .width(Length::Fill)
                    .padding(10)
                    .style(primary_button_style(palette, metrics.radius_sm)),
                );
            }

            container(pkce_col)
                .padding(16)
                .width(Length::Fill)
                .style(move |_| palette.card_container_style(metrics.radius_md))
                .into()
        }
        AuthTab::DeviceCode => {
            if let Some(ref dev) = state.device_auth {
                container(
                    column![
                        text(loc.t("auth.device_title"))
                            .font(FONT_UI_BOLD)
                            .size(metrics.font_heading),
                        text(&dev.verification_url)
                            .font(FONT_MONO_BOLD)
                            .size(metrics.font_body)
                            .color(palette.accent),
                        activation_code_box(&dev.user_code, palette, metrics),
                        text(format!(
                            "{}  ({}s)",
                            dev.status_message, dev.remaining_secs
                        ))
                        .size(metrics.font_caption)
                        .color(palette.text_secondary),
                        row![
                            button(
                                text(loc.t("auth.open_device_url"))
                                    .font(FONT_UI_SEMIBOLD)
                                    .size(metrics.font_caption)
                            )
                            .on_press(UiMessage::OpenDeviceUrl)
                            .padding([8, 16])
                            .style(primary_button_style(palette, 999.0)),
                            button(text(loc.t("auth.copy_code")).size(metrics.font_caption))
                                .on_press(UiMessage::CopyDeviceCode)
                                .padding([8, 14])
                                .style(secondary_button_style(palette, 999.0)),
                            Space::with_width(Length::Fill),
                            button(text(loc.t("auth.cancel_device")).size(metrics.font_caption))
                                .on_press(UiMessage::CancelDeviceLogin)
                                .padding([8, 14])
                                .style(secondary_button_style(palette, 999.0)),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    ]
                    .spacing(12)
                    .align_x(Alignment::Center),
                )
                .padding(16)
                .width(Length::Fill)
                .style(move |_| palette.card_container_style(metrics.radius_md))
                .into()
            } else {
                container(
                    column![
                        text(loc.t("auth.device_title"))
                            .font(FONT_UI_BOLD)
                            .size(metrics.font_heading),
                        text(loc.t("auth.device_steps"))
                            .size(metrics.font_caption)
                            .color(palette.text_secondary),
                        button(
                            text(if state.device_auth_loading {
                                "...".to_string()
                            } else {
                                loc.t("auth.request_code")
                            })
                            .font(FONT_UI_SEMIBOLD)
                            .size(metrics.font_body)
                        )
                        .on_press(UiMessage::StartDeviceLogin)
                        .padding([10, 20])
                        .width(Length::Fill)
                        .style(primary_button_style(palette, metrics.radius_sm)),
                    ]
                    .spacing(12),
                )
                .padding(16)
                .width(Length::Fill)
                .style(move |_| palette.card_container_style(metrics.radius_md))
                .into()
            }
        }
    };

    let mut modal_col = Column::new()
        .spacing(metrics.space_md)
        .push(header);

    if let Some(ref acc) = state.active_account {
        let tint = deterministic_tint(&acc.name);
        let ch_line = state
            .active_user_channel
            .as_ref()
            .map(|c| {
                format!(
                    "Channel: {} ({} subscribers)",
                    c.title,
                    c.formatted_subscriber_count()
                )
            })
            .unwrap_or_else(|| format!("Status: {}", state.auth_session_status));
        let pic_widget: Element<'a, UiMessage> = match acc
            .picture
            .as_ref()
            .and_then(|url| state.thumbnails.get(url))
        {
            Some(handle) => container(
                image(handle.clone())
                    .width(Length::Fixed(38.0))
                    .height(Length::Fixed(38.0))
                    .content_fit(iced::ContentFit::Cover),
            )
            .width(Length::Fixed(38.0))
            .height(Length::Fixed(38.0))
            .clip(true)
            .into(),
            None => avatar_circle(&acc.name, tint, 38.0).into(),
        };
        modal_col = modal_col.push(
            container(
                row![
                    pic_widget,
                    column![
                        text(format!("{} ({})", acc.name, acc.email))
                            .font(FONT_UI_SEMIBOLD)
                            .size(metrics.font_body),
                        text(ch_line)
                            .size(metrics.font_micro)
                            .color(palette.text_secondary),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .padding(12)
            .width(Length::Fill)
            .style(move |_| palette.card_container_style(metrics.radius_md)),
        );
    }

    modal_col = modal_col.push(auth_tabs).push(auth_panel);

    if state.show_credentials_form {
        modal_col = modal_col.push(
            container(
                column![
                    text(loc.t("auth.credentials_title"))
                        .font(FONT_UI_SEMIBOLD)
                        .size(metrics.font_caption),
                    text_input("GOOGLE_CLIENT_ID (...apps.googleusercontent.com)", &state.cred_client_id)
                        .on_input(UiMessage::CredClientIdChanged)
                        .padding([7, 10])
                        .size(metrics.font_caption)
                        .style(input_style(palette, metrics.radius_sm)),
                    text_input("GOOGLE_CLIENT_SECRET (optional for Desktop PKCE)", &state.cred_client_secret)
                        .on_input(UiMessage::CredClientSecretChanged)
                        .secure(true)
                        .padding([7, 10])
                        .size(metrics.font_caption)
                        .style(input_style(palette, metrics.radius_sm)),
                    text_input("YOUTUBE_API_KEY (optional for guest public data)", &state.cred_api_key)
                        .on_input(UiMessage::CredApiKeyChanged)
                        .padding([7, 10])
                        .size(metrics.font_caption)
                        .style(input_style(palette, metrics.radius_sm)),
                    row![
                        button(
                            text(loc.t("auth.save_credentials"))
                                .font(FONT_UI_SEMIBOLD)
                                .size(metrics.font_caption)
                        )
                        .on_press(UiMessage::SaveGoogleCredentials)
                        .padding([7, 14])
                        .style(primary_button_style(palette, metrics.radius_sm)),
                        button(text(loc.t("auth.import_json")).size(metrics.font_caption))
                            .on_press(UiMessage::ImportOAuthJson)
                            .padding([7, 14])
                            .style(secondary_button_style(palette, metrics.radius_sm)),
                    ]
                    .spacing(8),
                ]
                .spacing(8),
            )
            .padding(14)
            .width(Length::Fill)
            .style(move |_| palette.card_container_style(metrics.radius_md)),
        );
    }

    modal_col = modal_col.push(
        button(text(loc.t("auth.guest")).size(metrics.font_body))
            .on_press(UiMessage::SwitchToAccount(None))
            .width(Length::Fill)
            .padding(10)
            .style(secondary_button_style(palette, metrics.radius_sm)),
    );

    for acc in &state.accounts {
        let tint = deterministic_tint(&acc.name);
        modal_col = modal_col.push(
            row![
                button(
                    row![
                        avatar_circle(&acc.name, tint, 26.0),
                        text(format!("{} ({})", acc.name, acc.email)).size(metrics.font_body),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .on_press(UiMessage::SwitchToAccount(Some(acc.id.clone())))
                .width(Length::Fill)
                .padding(8)
                .style(secondary_button_style(palette, metrics.radius_sm)),
                button(text(loc.t("auth.logout")).size(metrics.font_micro))
                    .on_press(UiMessage::SignOutAccount(acc.id.clone()))
                    .padding([8, 10])
                    .style(secondary_button_style(palette, metrics.radius_sm)),
                button(text(loc.t("auth.revoke_access")).size(metrics.font_micro))
                    .on_press(UiMessage::RevokeGoogleAccess(acc.id.clone()))
                    .padding([8, 10])
                    .style(secondary_button_style(palette, metrics.radius_sm)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }

    container(
        container(scrollable(modal_col).height(Length::Shrink))
            .width(Length::Fixed(580.0))
            .max_height(660.0)
            .padding(22)
            .style(move |_| panel_style(palette, palette.bg_elevated, metrics.radius_lg)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(move |_| container::Style {
        background: Some(Background::Color(Color {
            a: 0.72,
            ..Color::BLACK
        })),
        ..Default::default()
    })
    .into()
}
