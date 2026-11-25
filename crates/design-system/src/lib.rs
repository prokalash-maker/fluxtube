pub mod components;
pub mod tokens;

pub use components::{
    danger_button_style, empty_state, error_banner, ghost_button_style, input_style, panel_style,
    pill_badge, primary_button_style, secondary_button_style, stat_tile, toast_banner,
};
pub use tokens::{parse_hex_color, DensityMode, LayoutMetrics, ThemeKind, ThemePalette};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_presets_and_hex_parsing() {
        let dark = ThemePalette::resolve(ThemeKind::GraphiteDark, "#FF0033", true);
        assert!(dark.is_dark);
        let light = ThemePalette::resolve(ThemeKind::ArcticLight, "#0066FF", true);
        assert!(!light.is_dark);
        let black = ThemePalette::resolve(ThemeKind::PureBlack, "#FF0033", true);
        assert_eq!(black.bg_primary, iced::Color::BLACK);

        let sys_light = ThemePalette::resolve(ThemeKind::System, "#FF0033", false);
        assert!(!sys_light.is_dark);

        assert!(parse_hex_color("#112233").is_some());
        assert!(parse_hex_color("invalid").is_none());
    }

    #[test]
    fn test_responsive_grid_columns_across_resolutions() {
        let m = LayoutMetrics::new(DensityMode::Standard, 1.0);
        // 1280x720 minus sidebar (~224px) => ~1020px => 3 columns
        assert!(m.grid_columns(1020.0) >= 3);
        // 1920x1080 minus sidebar => ~1660px => 5 columns
        assert!(m.grid_columns(1660.0) >= 5);
        // 4K => 6 columns max
        assert_eq!(m.grid_columns(3400.0), 6);
        // Narrow window => 1 column minimum
        assert_eq!(m.grid_columns(320.0), 1);
    }
}
