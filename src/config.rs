struct Config {
    // None means no database configured yet; login will show an error saying so.
    default_database: Option<PathBuf>,
    auto_lock: Duration,
    clipboard_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_database: None,
            auto_lock: Duration::from_secs(300),
            clipboard_timeout: Duration::from_secs(15),
        }
    }
}

#[derive(Deserialize, Default)]
struct ConfigFile {
    default_database: Option<String>,
    auto_lock: Option<u64>,
    clipboard_timeout: Option<u64>,
}

fn load_config() -> anyhow::Result<Config> {
    let path = expand_tilde("~/.config/puma/config.toml");
    let text =
        fs::read_to_string(&path).with_context(|| format!("couldn't read {}", path.display()))?;
    let raw: ConfigFile = toml::from_str(&text)?;

    let defaults = Config::default();

    Ok(Config {
        default_database: raw.default_database.map(|s| expand_tilde(&s)),
        auto_lock: raw
            .auto_lock
            .map(Duration::from_secs)
            .unwrap_or(defaults.auto_lock),
        clipboard_timeout: raw
            .clipboard_timeout
            .map(Duration::from_secs)
            .unwrap_or(defaults.clipboard_timeout),
    })
}
