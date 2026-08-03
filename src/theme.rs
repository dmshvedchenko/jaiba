pub struct Theme {
    background: Color,
    text: Color,
    warning: Color,
    error: Color,
    success: Color,
    border: Color,
    header: Color,
    accent: Color,
    selection_fg: Color,
    selection_bg: Color,
}

#[derive(Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    pub colors: ThemeColors,
}

#[derive(Deserialize)]
pub struct ThemeColors {
    pub background: String,
    pub text: String,

    pub border: String,
    pub header: String,

    pub accent: String,

    pub warning: String,
    pub error: String,
    pub success: String,

    pub selection_fg: String,
    pub selection_bg: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Color::Rgb(30, 30, 46),
            text: Color::Rgb(205, 214, 244),
            warning: Color::Rgb(249, 226, 175),
            error: Color::Rgb(243, 139, 168),
            success: Color::Rgb(166, 227, 161),
            border: Color::Rgb(69, 71, 90),
            header: Color::Rgb(147, 153, 178),
            accent: Color::Rgb(203, 166, 247),
            selection_fg: Color::Rgb(30, 30, 46),
            selection_bg: Color::Rgb(137, 180, 250),
        }
    }
}

impl TryFrom<ThemeConfig> for Theme {
    type Error = anyhow::Error;

    fn try_from(cfg: ThemeConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            background: parse_hex(&cfg.colors.background)?,
            text: parse_hex(&cfg.colors.text)?,

            border: parse_hex(&cfg.colors.border)?,
            header: parse_hex(&cfg.colors.header)?,

            accent: parse_hex(&cfg.colors.accent)?,

            warning: parse_hex(&cfg.colors.warning)?,
            error: parse_hex(&cfg.colors.error)?,
            success: parse_hex(&cfg.colors.success)?,

            selection_fg: parse_hex(&cfg.colors.selection_fg)?,
            selection_bg: parse_hex(&cfg.colors.selection_bg)?,
        })
    }
}

pub fn load_theme() -> anyhow::Result<Theme> {
    let path = expand_tilde("~/.config/puma/theme.toml");
    let text = fs::read_to_string(path)?;
    let config: ThemeConfig = toml::from_str(&text)?;
    Theme::try_from(config)
}

fn parse_hex(hex: &str) -> anyhow::Result<Color> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);

    if hex.len() != 6 {
        anyhow::bail!("expected 6 hex digits");
    }

    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;

    Ok(Color::Rgb(r, g, b))
}
