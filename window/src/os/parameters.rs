use config::RgbColor;
use wezterm_font::parser::ParsedFont;

pub struct TitleBar {
    pub padding_left: f32,
    pub padding_right: f32,
    pub height: Option<f32>,
    pub font_and_size: Option<(ParsedFont, f64)>,
}

pub struct Border {
    pub top: f32,
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub color: RgbColor,
}

pub struct Parameters {
    pub title_bar: TitleBar,
    pub border_dimensions: Option<Border>, // If present, the application should draw it
}
