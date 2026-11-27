//! サイトのファビコンの取得（サーバーが代わりに取る。友達の画面で、サイトの名前の横に出す）。
//!
//! 画面の側で取らせない理由: 友達が見たサイトに、見ている人の接続元が知られるため。
//! サーバーが取るので、内部のアドレスを取りに行かせないよう、次を守る。
//! - `https` と標準ポートだけ。ホストは、数字だけでない通常のドメイン名だけ（IP アドレス・`.local` などは取らない）
//! - 名前を引いたアドレスが 1 つでも公開されていないもの（ループバック・プライベート・Tailscale の CGNAT など）なら取らない
//! - 引いたアドレスに接続を固定する（取る直前と接続のあいだに、別のアドレスに差し替えられないように）
//! - リダイレクトは自分で最大 3 回までたどり、行き先も同じ確認をする
//! - 画像と確かめられた（先頭のバイト列）ものだけ受け取る。SVG は受け取らない。大きさは 128KB まで

use std::net::{IpAddr, SocketAddr};
use std::sync::OnceLock;
use std::time::Duration;

use reqwest::Url;
use tokio::sync::Semaphore;

const MAX_ICON_BYTES: usize = 128 * 1024;
const MAX_HTML_BYTES: usize = 256 * 1024;
const MAX_REDIRECTS: usize = 3;
const OVERALL_TIMEOUT: Duration = Duration::from_secs(12);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// 同時に取る数（サーバーが外へ取りに行く量を抑える）。
fn permits() -> &'static Semaphore {
    static S: OnceLock<Semaphore> = OnceLock::new();
    S.get_or_init(|| Semaphore::new(4))
}

/// 通常のドメイン名か（小文字の ASCII。IP アドレスや、内部向けの名前は除く）。
pub fn valid_domain(d: &str) -> bool {
    if d.len() > 253 || !d.contains('.') {
        return false;
    }
    let labels_ok = d.split('.').all(|l| {
        !l.is_empty()
            && l.len() <= 63
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    });
    let tld = d.rsplit('.').next().unwrap_or("");
    labels_ok
        && !tld.bytes().all(|b| b.is_ascii_digit())
        && !matches!(tld, "local" | "localhost" | "internal" | "lan" | "home" | "corp" | "intranet" | "ts")
}

/// 公開されたアドレスか。
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => {
            let o = a.octets();
            !(a.is_private()
                || a.is_loopback()
                || a.is_link_local()
                || a.is_broadcast()
                || a.is_unspecified()
                || a.is_documentation()
                || a.is_multicast()
                || o[0] == 0
                || o[0] >= 240
                || (o[0] == 100 && (o[1] & 0xc0) == 64) // 100.64.0.0/10（CGNAT。Tailscale もここ）
                || (o[0] == 198 && (o[1] & 0xfe) == 18)) // 198.18.0.0/15
        }
        IpAddr::V6(a) => {
            if let Some(v4) = a.to_ipv4_mapped() {
                return is_public(IpAddr::V4(v4));
            }
            let s0 = a.segments()[0];
            !(a.is_loopback()
                || a.is_unspecified()
                || a.is_multicast()
                || (s0 & 0xfe00) == 0xfc00 // fc00::/7
                || (s0 & 0xffc0) == 0xfe80) // fe80::/10
        }
    }
}

/// 先頭のバイト列から画像の種類を判定する（SVG は含めない）。
pub fn sniff(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Some("image/png")
    } else if b.starts_with(&[0, 0, 1, 0]) {
        Some("image/x-icon")
    } else if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if b.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if b.len() > 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// タグの属性の値（名前は大文字小文字を区別しない。引用符あり・なし）。
fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let at = from + i;
        from = at + name.len();
        let before_ok = at > 0 && lower.as_bytes()[at - 1].is_ascii_whitespace();
        let rest = lower[from..].trim_start();
        if !before_ok || !rest.starts_with('=') {
            continue;
        }
        // 値の始まり（元の文字列の位置で切り出す）
        let after_eq = from + (lower[from..].len() - rest.len()) + 1;
        let value = tag[after_eq..].trim_start();
        return Some(match value.chars().next()? {
            q @ ('"' | '\'') => value[1..].split(q).next()?.to_owned(),
            _ => value.split(|c: char| c.is_whitespace() || c == '>').next()?.to_owned(),
        });
    }
    None
}

/// HTML の `<link rel="icon" href="...">` から、アイコンの場所（SVG は除く）。
pub fn find_icon_href(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let mut pos = 0;
    while let Some(i) = lower[pos..].find("<link") {
        let start = pos + i;
        let end = lower[start..].find('>').map_or(lower.len(), |e| start + e);
        pos = end;
        let tag = &html[start..end];
        let is_icon = attr(tag, "rel").is_some_and(|r| r.to_ascii_lowercase().split_whitespace().any(|t| t == "icon"));
        if !is_icon {
            continue;
        }
        let Some(href) = attr(tag, "href").map(|h| h.trim().to_owned()).filter(|h| !h.is_empty()) else {
            continue;
        };
        let svg = attr(tag, "type").is_some_and(|t| t.contains("svg"))
            || href.to_ascii_lowercase().split(['?', '#']).next().is_some_and(|p| p.ends_with(".svg"));
        if !svg {
            return Some(href);
        }
    }
    None
}

/// 公開されたアドレスだけに名前が引けるか。引けたら接続先のアドレスを返す。
async fn resolve_public(host: &str) -> Option<SocketAddr> {
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host, 443)).await.ok()?.collect();
    if addrs.is_empty() || addrs.iter().any(|a| !is_public(a.ip())) {
        return None;
    }
    addrs.first().copied()
}

/// 1 つの URL を取る（リダイレクトは、行き先も確かめながら自分でたどる）。取れた最終の URL と本文を返す。
async fn get(mut url: Url, limit: usize) -> Option<(Url, Vec<u8>)> {
    for _ in 0..=MAX_REDIRECTS {
        let host = match url.host()? {
            url::Host::Domain(d) => d.to_ascii_lowercase(),
            _ => return None,
        };
        if url.scheme() != "https" || url.port().is_some() || !valid_domain(&host) {
            return None;
        }
        let addr = resolve_public(&host).await?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(REQUEST_TIMEOUT)
            .user_agent("Mozilla/5.0 (compatible; offsave-favicon)")
            .resolve(&host, addr)
            .build()
            .ok()?;
        let mut res = client.get(url.clone()).send().await.ok()?;
        if res.status().is_redirection() {
            let loc = res.headers().get(reqwest::header::LOCATION)?.to_str().ok()?;
            url = url.join(loc).ok()?;
            continue;
        }
        if !res.status().is_success() {
            return None;
        }
        let mut body = Vec::new();
        while let Some(chunk) = res.chunk().await.ok()? {
            if body.len() + chunk.len() > limit {
                return None;
            }
            body.extend_from_slice(&chunk);
        }
        return Some((url, body));
    }
    None
}

async fn get_image(url: Url) -> Option<(&'static str, Vec<u8>)> {
    let (_, body) = get(url, MAX_ICON_BYTES).await?;
    Some((sniff(&body)?, body))
}

async fn fetch_inner(domain: &str) -> Option<(&'static str, Vec<u8>)> {
    let root = Url::parse(&format!("https://{domain}/")).ok()?;
    if let Some(found) = get_image(root.join("/favicon.ico").ok()?).await {
        return Some(found);
    }
    // /favicon.ico が無いサイトは、ページの <link rel="icon"> から探す
    let (final_url, html) = get(root, MAX_HTML_BYTES).await?;
    let href = find_icon_href(&String::from_utf8_lossy(&html))?;
    get_image(final_url.join(&href).ok()?).await
}

/// ドメインのファビコン（画像の種類と中身）。取れなければ None。
pub async fn fetch(domain: &str) -> Option<(&'static str, Vec<u8>)> {
    if !valid_domain(domain) {
        return None;
    }
    let _permit = permits().acquire().await.ok()?;
    tokio::time::timeout(OVERALL_TIMEOUT, fetch_inner(domain)).await.ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_ordinary_public_domains_are_fetched() {
        for d in ["example.com", "sub.example.co.jp", "a-b.example.org", "xn--eckwd4c7c.jp"] {
            assert!(valid_domain(d), "{d}");
        }
        for d in [
            "localhost", "intranet", "127.0.0.1", "10.0.0.1", "100.83.29.24", "foo.local", "a.internal", "-a.com", "a..com",
            "exa mple.com", "EXAMPLE.com", "a.com:8080", "foo.ts", "a/b.com", "",
        ] {
            assert!(!valid_domain(d), "{d}");
        }
    }

    #[test]
    fn private_and_special_addresses_are_not_public() {
        for ip in [
            "127.0.0.1", "10.1.2.3", "172.16.0.1", "192.168.1.1", "169.254.169.254", "100.64.0.1", "100.127.255.255",
            "0.0.0.0", "224.0.0.1", "255.255.255.255", "198.18.0.1", "240.0.0.1", "::1", "::", "fc00::1", "fd12::1", "fe80::1",
            "::ffff:127.0.0.1", "::ffff:100.83.29.24", "ff02::1",
        ] {
            assert!(!is_public(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["93.184.216.34", "8.8.8.8", "100.128.0.1", "172.32.0.1", "2606:2800:220:1:248:1893:25c8:1946"] {
            assert!(is_public(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn images_are_recognised_by_their_bytes() {
        assert_eq!(sniff(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0]), Some("image/png"));
        assert_eq!(sniff(&[0, 0, 1, 0, 1, 0]), Some("image/x-icon"));
        assert_eq!(sniff(b"GIF89a..."), Some("image/gif"));
        assert_eq!(sniff(&[0xff, 0xd8, 0xff, 0xe0]), Some("image/jpeg"));
        assert_eq!(sniff(b"RIFF\x00\x00\x00\x00WEBPVP8 "), Some("image/webp"));
        assert_eq!(sniff(b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>"), None);
        assert_eq!(sniff(b"<!doctype html><html>"), None);
        assert_eq!(sniff(b""), None);
    }

    #[test]
    fn icon_link_is_found_in_html() {
        let html = r#"<html><head><LINK REL="stylesheet" href="/a.css">
            <link rel="mask-icon" href="/mask.svg">
            <link rel="icon" type="image/svg+xml" href="/icon.svg">
            <link href='/fav.png?v=2' rel='shortcut icon' type="image/png">
            </head>"#;
        assert_eq!(find_icon_href(html).as_deref(), Some("/fav.png?v=2"));
        assert_eq!(find_icon_href("<link rel=icon href=//cdn.example.com/x.ico>").as_deref(), Some("//cdn.example.com/x.ico"));
        assert_eq!(find_icon_href("<link rel=\"icon\" href=\"/only.svg\">"), None);
        assert_eq!(find_icon_href("<p>no icons</p>"), None);
        // 属性名の一部（data-rel など）には反応しない
        assert_eq!(find_icon_href("<link data-rel=\"icon\" href=\"/x.png\">"), None);
    }
}
