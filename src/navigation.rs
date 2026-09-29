use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use url::Url;

#[derive(Clone, Debug)]
pub struct NavigationPolicy {
    main_url: Url,
}

impl NavigationPolicy {
    pub fn new(main_url: &str) -> Result<Self, url::ParseError> {
        Ok(Self {
            main_url: Url::parse(main_url)?,
        })
    }

    pub fn is_internal(&self, url: &Url) -> bool {
        if matches!(url.scheme(), "about" | "chrome") {
            return true;
        }

        let Some(hostname) = url.host_str() else {
            return false;
        };

        if matches!(hostname, "localhost" | "127.0.0.1") {
            return true;
        }

        url.scheme() == "https"
            && matches!(
                hostname,
                "telemost.360.yandex.ru"
                    | "telemost.yandex.ru"
                    | "passport.yandex.ru"
                    | "id.yandex.ru"
                    | "cookier.360.yandex.ru"
            )
    }

    pub fn is_main(&self, url: &Url) -> bool {
        url.scheme() == "https"
            && (url.host() == self.main_url.host() || url.host_str() == Some("telemost.yandex.ru"))
            && url.port().is_none()
            && url.username().is_empty()
            && url.password().is_none()
            && normalize_pathname(url.path()) == normalize_pathname(self.main_url.path())
    }

    pub fn open_external(&self, app: &AppHandle, url: &Url) -> Result<(), String> {
        app.opener()
            .open_url(url.as_str(), None::<&str>)
            .map_err(|error| {
                format!("failed to open external navigation in the default browser: {error}")
            })
    }
}

pub fn normalize_pathname(pathname: &str) -> &str {
    let normalized = pathname.trim_end_matches('/');
    if normalized.is_empty() {
        "/"
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_navigation_keeps_only_telemost_and_auth_hosts_inside() {
        let policy = NavigationPolicy::new(crate::config::TELEMOST_URL).unwrap();
        for host in [
            "telemost.360.yandex.ru",
            "telemost.yandex.ru",
            "passport.yandex.ru",
            "id.yandex.ru",
            "cookier.360.yandex.ru",
        ] {
            assert!(policy.is_internal(&Url::parse(&format!("https://{host}/login")).unwrap()));
        }

        for url in [
            "https://evil.telemost.360.yandex.ru/",
            "https://telemost.360.yandex.ru.evil.test/",
            "https://metrika.yandex.ru/",
            "http://passport.yandex.ru/",
        ] {
            assert!(!policy.is_internal(&Url::parse(url).unwrap()), "{url}");
        }
        assert!(policy.is_main(&Url::parse(crate::config::TELEMOST_URL).unwrap()));
    }

    #[test]
    fn unread_tracking_accepts_both_telemost_roots_only() {
        let policy = NavigationPolicy::new(crate::config::TELEMOST_URL).unwrap();
        for raw in [
            "https://telemost.360.yandex.ru/?skip_app=1#/chats/example",
            "https://telemost.yandex.ru/?skip_app=1#/threads",
        ] {
            assert!(policy.is_main(&Url::parse(raw).unwrap()), "{raw}");
        }
        for raw in [
            "http://telemost.360.yandex.ru/",
            "https://telemost.360.yandex.ru:8443/",
            "https://user@telemost.360.yandex.ru/",
            "https://telemost.yandex.ru.evil.test/",
            "https://passport.yandex.ru/",
            "https://telemost.yandex.ru/j/123",
        ] {
            assert!(!policy.is_main(&Url::parse(raw).unwrap()), "{raw}");
        }
    }
}
