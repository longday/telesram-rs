use url::Url;

pub fn decode(input: &str) -> Result<Url, &'static str> {
    if input
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control() || byte == b'\\')
    {
        return Err("invalid characters in deep link");
    }
    let (scheme, payload) = input.split_once("://").ok_or("invalid deep link")?;
    if !scheme.eq_ignore_ascii_case("telemost") {
        return Err("unsupported deep link scheme");
    }
    let (destination, meeting) = if payload.starts_with("https://") {
        (Url::parse(payload), true)
    } else if let Some(rest) = payload.strip_prefix("https//") {
        // Tauri's Url parser removes the empty port colon from telemost://https:// URLs.
        (Url::parse(&format!("https://{rest}")), true)
    } else if let Some(rest) = payload.strip_prefix("ychat/") {
        (Url::parse(&format!("https://{rest}")), false)
    } else {
        return Err("unsupported deep link format");
    };
    let mut destination = destination.map_err(|_| "invalid destination URL")?;
    if destination.scheme() != "https"
        || !matches!(
            destination.host_str(),
            Some("telemost.yandex.ru" | "telemost.360.yandex.ru")
        )
        || !destination.username().is_empty()
        || destination.password().is_some()
        || destination.port().is_some()
    {
        return Err("unsupported deep link destination");
    }
    if meeting {
        let id = destination
            .path()
            .strip_prefix("/j/")
            .ok_or("invalid meeting path")?;
        if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err("invalid meeting ID");
        }
    }
    if destination.query_pairs().any(|(key, _)| key == "skip_app") {
        let original = destination.query().unwrap_or_default();
        let mut query = String::with_capacity(original.len() + "&skip_app=1".len());
        for part in original.split('&') {
            if url::form_urlencoded::parse(part.as_bytes())
                .next()
                .is_some_and(|(key, _)| key == "skip_app")
            {
                continue;
            }
            query.push_str(part);
            query.push('&');
        }
        query.push_str("skip_app=1");
        destination.set_query(Some(&query));
    } else {
        destination.query_pairs_mut().append_pair("skip_app", "1");
    }
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::decode;
    use url::Url;

    #[test]
    fn meeting_links_survive_native_url_normalization() {
        for host in ["telemost.yandex.ru", "telemost.360.yandex.ru"] {
            let raw = format!("telemost://https://{host}/j/123456789");
            let expected = format!("https://{host}/j/123456789?skip_app=1");
            assert_eq!(decode(&raw).unwrap().as_str(), expected);
            let native = Url::parse(&raw).unwrap();
            assert_eq!(decode(native.as_str()).unwrap().as_str(), expected);
        }
    }

    #[test]
    fn chat_route_preserves_parameters_and_fragment_without_relaunch() {
        let url = decode("telemost://ychat/telemost.360.yandex.ru/?source=calendar&skip_app=0&skip_app=2#/chats/a%2Fb").unwrap();
        assert_eq!(
            url.as_str(),
            "https://telemost.360.yandex.ru/?source=calendar&skip_app=1#/chats/a%2Fb"
        );
        let url = decode("telemost://ychat/telemost.yandex.ru/?source=a%26b#/join/invite").unwrap();
        assert_eq!(
            url.as_str(),
            "https://telemost.yandex.ru/?source=a%26b&skip_app=1#/join/invite"
        );
    }

    #[test]
    fn replacing_skip_app_preserves_other_query_bytes() {
        let url = decode("telemost://ychat/telemost.yandex.ru/?next=/chats/x&flag&text=a%20b&skip%5Fapp=0&skip_app=2#route").unwrap();
        assert_eq!(
            url.as_str(),
            "https://telemost.yandex.ru/?next=/chats/x&flag&text=a%20b&skip_app=1#route"
        );
    }

    #[test]
    fn hostile_or_malformed_destinations_are_rejected() {
        for raw in [
            "https://telemost.yandex.ru/j/1",
            "ychat://telemost.yandex.ru/",
            "telemost://http://telemost.yandex.ru/j/1",
            "telemost://https://telemost.yandex.ru.evil.test/j/1",
            "telemost://https://evil.telemost.yandex.ru/j/1",
            "telemost://https://user@telemost.yandex.ru/j/1",
            "telemost://https://:secret@telemost.yandex.ru/j/1",
            "telemost://https://telemost.yandex.ru:8443/j/1",
            "telemost://https://telemost.yandex.ru/j/",
            "telemost://https://telemost.yandex.ru/j/abc",
            "telemost://https://telemost.yandex.ru/j/1/extra",
            "telemost://https://telemost.yandex.ru/other/1",
            "telemost://ychat/evil.test/#/chats/x",
            "telemost://ychat/passport.yandex.ru/",
            "telemost://ychat/telemost.yandex.ru@evil.test/",
            "telemost://ychat/user:secret@telemost.yandex.ru/",
            "telemost://ychat/:secret@telemost.yandex.ru/",
            "telemost://ychat/telemost.yandex.ru:8443/",
            "telemost://ychat/telemost.yandex.ru\\@evil.test/",
            "telemost://ychat/telemost.yandex.ru\n/",
            "telemost://ychat/",
            "telemost://",
        ] {
            assert!(decode(raw).is_err(), "{raw}");
        }
    }
}
