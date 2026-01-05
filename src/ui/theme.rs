use ratatui::style::Color;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeVariant {
    Default,
    CatppuccinLatte,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    CatppuccinMocha,
    Nord,
}

impl ThemeVariant {
    pub fn next(&self) -> Self {
        match self {
            ThemeVariant::Default => ThemeVariant::CatppuccinLatte,
            ThemeVariant::CatppuccinLatte => ThemeVariant::CatppuccinFrappe,
            ThemeVariant::CatppuccinFrappe => ThemeVariant::CatppuccinMacchiato,
            ThemeVariant::CatppuccinMacchiato => ThemeVariant::CatppuccinMocha,
            ThemeVariant::CatppuccinMocha => ThemeVariant::Nord,
            ThemeVariant::Nord => ThemeVariant::Default,
        }
    }

    pub fn all() -> &'static [ThemeVariant] {
        &[
            ThemeVariant::Default,
            ThemeVariant::CatppuccinLatte,
            ThemeVariant::CatppuccinFrappe,
            ThemeVariant::CatppuccinMacchiato,
            ThemeVariant::CatppuccinMocha,
            ThemeVariant::Nord,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ThemeVariant::Default => "默认暗色",
            ThemeVariant::CatppuccinLatte => "Catppuccin Latte",
            ThemeVariant::CatppuccinFrappe => "Catppuccin Frappé",
            ThemeVariant::CatppuccinMacchiato => "Catppuccin Macchiato",
            ThemeVariant::CatppuccinMocha => "Catppuccin Mocha",
            ThemeVariant::Nord => "Nord",
        }
    }
}

impl fmt::Display for ThemeVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ThemeVariant::Default => "Default",
            ThemeVariant::CatppuccinLatte => "CatppuccinLatte",
            ThemeVariant::CatppuccinFrappe => "CatppuccinFrappe",
            ThemeVariant::CatppuccinMacchiato => "CatppuccinMacchiato",
            ThemeVariant::CatppuccinMocha => "CatppuccinMocha",
            ThemeVariant::Nord => "Nord",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for ThemeVariant {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Default" => Ok(ThemeVariant::Default),
            "CatppuccinLatte" => Ok(ThemeVariant::CatppuccinLatte),
            "CatppuccinFrappe" => Ok(ThemeVariant::CatppuccinFrappe),
            "CatppuccinMacchiato" => Ok(ThemeVariant::CatppuccinMacchiato),
            "CatppuccinMocha" => Ok(ThemeVariant::CatppuccinMocha),
            "Nord" => Ok(ThemeVariant::Nord),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_modal: Color,

    pub fg_primary: Color,
    pub fg_secondary: Color,
    pub fg_accent: Color,

    pub border_focused: Color,
    pub border_unfocused: Color,

    pub selection_bg: Color,
    pub selection_fg: Color,

    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    pub bilibili_pink: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_variant(ThemeVariant::Default)
    }
}

impl Theme {
    pub fn from_variant(variant: ThemeVariant) -> Self {
        match variant {
            ThemeVariant::Default => Self::default_theme(),
            ThemeVariant::CatppuccinLatte => Self::catppuccin_latte(),
            ThemeVariant::CatppuccinFrappe => Self::catppuccin_frappe(),
            ThemeVariant::CatppuccinMacchiato => Self::catppuccin_macchiato(),
            ThemeVariant::CatppuccinMocha => Self::catppuccin_mocha(),
            ThemeVariant::Nord => Self::nord(),
        }
    }

    fn from_catppuccin(c: catppuccin::Color) -> Color {
        Color::Rgb(c.rgb.r, c.rgb.g, c.rgb.b)
    }

    fn default_theme() -> Self {
        // Default Bilibili-like Dark Theme (approximation of current hardcoded colors)
        Self {
            bg_primary: Color::Reset,
            bg_secondary: Color::Rgb(40, 40, 40),
            bg_modal: Color::Rgb(30, 30, 30),

            fg_primary: Color::White,
            fg_secondary: Color::Rgb(150, 150, 150),
            fg_accent: Color::Cyan,

            border_focused: Color::Cyan, // Or Bilibili Pink
            border_unfocused: Color::Rgb(50, 50, 50),

            selection_bg: Color::Rgb(60, 60, 60),
            selection_fg: Color::White,

            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Cyan,

            bilibili_pink: Color::Rgb(251, 114, 153),
        }
    }

    fn catppuccin_latte() -> Self {
        let p = catppuccin::PALETTE.latte.colors;
        Self {
            bg_primary: Self::from_catppuccin(p.base),
            bg_secondary: Self::from_catppuccin(p.mantle),
            bg_modal: Self::from_catppuccin(p.crust),

            fg_primary: Self::from_catppuccin(p.text),
            fg_secondary: Self::from_catppuccin(p.subtext1),
            fg_accent: Self::from_catppuccin(p.blue),

            border_focused: Self::from_catppuccin(p.mauve),
            border_unfocused: Self::from_catppuccin(p.surface2),

            selection_bg: Self::from_catppuccin(p.surface2),
            selection_fg: Self::from_catppuccin(p.text),

            success: Self::from_catppuccin(p.green),
            warning: Self::from_catppuccin(p.yellow),
            error: Self::from_catppuccin(p.red),
            info: Self::from_catppuccin(p.sky),

            bilibili_pink: Self::from_catppuccin(p.pink),
        }
    }

    fn catppuccin_frappe() -> Self {
        let p = catppuccin::PALETTE.frappe.colors;
        Self {
            bg_primary: Self::from_catppuccin(p.base),
            bg_secondary: Self::from_catppuccin(p.mantle),
            bg_modal: Self::from_catppuccin(p.crust),

            fg_primary: Self::from_catppuccin(p.text),
            fg_secondary: Self::from_catppuccin(p.subtext1),
            fg_accent: Self::from_catppuccin(p.blue),

            border_focused: Self::from_catppuccin(p.mauve),
            border_unfocused: Self::from_catppuccin(p.surface2),

            selection_bg: Self::from_catppuccin(p.surface2),
            selection_fg: Self::from_catppuccin(p.text),

            success: Self::from_catppuccin(p.green),
            warning: Self::from_catppuccin(p.yellow),
            error: Self::from_catppuccin(p.red),
            info: Self::from_catppuccin(p.sky),

            bilibili_pink: Self::from_catppuccin(p.pink),
        }
    }

    fn catppuccin_macchiato() -> Self {
        let p = catppuccin::PALETTE.macchiato.colors;
        Self {
            bg_primary: Self::from_catppuccin(p.base),
            bg_secondary: Self::from_catppuccin(p.mantle),
            bg_modal: Self::from_catppuccin(p.crust),

            fg_primary: Self::from_catppuccin(p.text),
            fg_secondary: Self::from_catppuccin(p.subtext1),
            fg_accent: Self::from_catppuccin(p.blue),

            border_focused: Self::from_catppuccin(p.mauve),
            border_unfocused: Self::from_catppuccin(p.surface2),

            selection_bg: Self::from_catppuccin(p.surface2),
            selection_fg: Self::from_catppuccin(p.text),

            success: Self::from_catppuccin(p.green),
            warning: Self::from_catppuccin(p.yellow),
            error: Self::from_catppuccin(p.red),
            info: Self::from_catppuccin(p.sky),

            bilibili_pink: Self::from_catppuccin(p.pink),
        }
    }

    fn catppuccin_mocha() -> Self {
        let p = catppuccin::PALETTE.mocha.colors;
        Self {
            bg_primary: Self::from_catppuccin(p.base),
            bg_secondary: Self::from_catppuccin(p.mantle),
            bg_modal: Self::from_catppuccin(p.crust),

            fg_primary: Self::from_catppuccin(p.text),
            fg_secondary: Self::from_catppuccin(p.subtext1),
            fg_accent: Self::from_catppuccin(p.blue),

            border_focused: Self::from_catppuccin(p.mauve),
            border_unfocused: Self::from_catppuccin(p.surface2),

            selection_bg: Self::from_catppuccin(p.surface2),
            selection_fg: Self::from_catppuccin(p.text),

            success: Self::from_catppuccin(p.green),
            warning: Self::from_catppuccin(p.yellow),
            error: Self::from_catppuccin(p.red),
            info: Self::from_catppuccin(p.sky),

            bilibili_pink: Self::from_catppuccin(p.pink),
        }
    }

    fn nord() -> Self {
        // Nord palette
        // https://www.nordtheme.com/docs/colors-and-palettes
        let nord0 = Color::Rgb(46, 52, 64); // Polar Night
        let nord1 = Color::Rgb(59, 66, 82);
        let nord2 = Color::Rgb(67, 76, 94);
        let nord3 = Color::Rgb(76, 86, 106);
        let nord4 = Color::Rgb(216, 222, 233); // Snow Storm
        let _nord5 = Color::Rgb(229, 233, 240);
        let nord6 = Color::Rgb(236, 239, 244);
        let _nord7 = Color::Rgb(143, 188, 187); // Frost
        let nord8 = Color::Rgb(136, 192, 208);
        let nord9 = Color::Rgb(129, 161, 193);
        let _nord10 = Color::Rgb(94, 129, 172);
        let nord11 = Color::Rgb(191, 97, 106); // Aurora (Red)
        let _nord12 = Color::Rgb(208, 135, 112); // Orange
        let nord13 = Color::Rgb(235, 203, 139); // Yellow
        let nord14 = Color::Rgb(163, 190, 140); // Green
        let nord15 = Color::Rgb(180, 142, 173); // Purple

        Self {
            bg_primary: nord0,
            bg_secondary: nord1,
            bg_modal: nord2,

            fg_primary: nord4,
            fg_secondary: nord3, // or nord4 dimmed
            fg_accent: nord8,

            border_focused: nord8,
            border_unfocused: nord3,

            selection_bg: nord2,
            selection_fg: nord6,

            success: nord14,
            warning: nord13,
            error: nord11,
            info: nord9,

            bilibili_pink: nord15, // Use default purple for pink slots
        }
    }
}
