use config::RgbColor;
use wezterm_font::parser::ParsedFont;

pub struct TitleBar {
    padding_left: f32,
    padding_right: f32,
    height: Option<f32>,
    font: Option<(ParsedFont, f64)>,
}

impl TitleBar {
    pub fn new(
        padding_left: f32,
        padding_right: f32,
        height: Option<f32>,
        font: Option<(ParsedFont, f64)>,
    ) -> Self {
        Self {
            padding_left,
            padding_right,
            height,
            font,
        }
    }

    pub fn padding_left(&self) -> f32 {
        self.padding_left
    }

    pub fn padding_right(&self) -> f32 {
        self.padding_right
    }

    pub fn height(&self) -> Option<f32> {
        self.height
    }

    pub fn font(&self) -> Option<&(ParsedFont, f64)> {
        self.font.as_ref()
    }
}

pub struct Border {
    top: f32,
    left: f32,
    bottom: f32,
    right: f32,
    color: RgbColor,
}

impl Border {
    pub fn new(top: f32, left: f32, bottom: f32, right: f32, color: RgbColor) -> Self {
        Self {
            top,
            left,
            bottom,
            right,
            color,
        }
    }

    pub fn top(&self) -> f32 {
        self.top
    }

    pub fn left(&self) -> f32 {
        self.left
    }

    pub fn bottom(&self) -> f32 {
        self.bottom
    }

    pub fn right(&self) -> f32 {
        self.right
    }

    pub fn color(&self) -> RgbColor {
        self.color
    }
}

pub struct Parameters {
    title_bar: TitleBar,
    window_border_to_draw: Option<Border>,
}

impl Parameters {
    pub fn new(title_bar: TitleBar, window_border_to_draw: Option<Border>) -> Self {
        Self {
            title_bar,
            window_border_to_draw,
        }
    }

    pub fn title_bar(&self) -> &TitleBar {
        &self.title_bar
    }

    pub fn window_border_to_draw(&self) -> Option<&Border> {
        self.window_border_to_draw.as_ref()
    }
}
