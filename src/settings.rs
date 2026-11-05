//! Настройки приложения (не мыши): `~/.config/ardor-mouse/settings.conf`,
//! строки `ключ = значение`. Пока там только язык интерфейса.

use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    /// Тег языка (`ru`, `en`…); `None` — как в системе.
    pub language: Option<String>,
}

fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("ardor-mouse").join("settings.conf"))
}

impl Settings {
    pub fn load() -> Self {
        path().and_then(|p| std::fs::read_to_string(p).ok()).map(|s| Self::parse(&s)).unwrap_or_default()
    }

    fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            if k.trim() == "language" {
                let v = v.trim();
                s.language = (!v.is_empty() && v != "system").then(|| v.to_string());
            }
        }
        s
    }

    fn render(&self) -> String {
        format!("language = {}\n", self.language.as_deref().unwrap_or("system"))
    }

    pub fn save(&self) {
        let Some(p) = path() else { return };
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&p, self.render()) {
            eprintln!("ardor-mouse: {}: {e}", p.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let s = Settings { language: Some("en".into()) };
        assert_eq!(Settings::parse(&s.render()), s);
        let sys = Settings::default();
        assert_eq!(Settings::parse(&sys.render()), sys);
        assert_eq!(Settings::parse("# мусор\nlanguage=ru\n").language.as_deref(), Some("ru"));
    }
}
