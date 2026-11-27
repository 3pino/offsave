//! 版の比較（`1.2.3` の形だけ。先頭の `v` は付いていてもよい）。プレリリース・ビルド情報は扱わない（解釈できないものは `None`）。

/// `v1.2.3` / `1.2.3` → (1, 2, 3)
pub fn parse(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().strip_prefix('v').unwrap_or(s.trim());
    let mut it = s.split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next()?.parse().ok()?;
    let patch = it.next()?.parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    // `01` や `+1` のような書き方は解釈しない（数字だけ）
    let digits = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
    s.split('.').all(digits).then_some((major, minor, patch))
}

/// `candidate` が `current` より新しいか。どちらかが解釈できなければ `None`。
pub fn is_newer(candidate: &str, current: &str) -> Option<bool> {
    Some(parse(candidate)? > parse(current)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_versions() {
        assert_eq!(parse("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse("v0.10.0"), Some((0, 10, 0)));
        assert_eq!(parse(" 2.0.1 "), Some((2, 0, 1)));
    }

    #[test]
    fn rejects_everything_else() {
        for s in ["", "1", "1.2", "1.2.3.4", "1.2.x", "1.2.3-rc.1", "1.2.3+b", "+1.2.3", "1..3", "-1.0.0"] {
            assert_eq!(parse(s), None, "{s}");
        }
    }

    #[test]
    fn compares_numerically() {
        assert_eq!(is_newer("0.10.0", "0.9.9"), Some(true));
        assert_eq!(is_newer("0.2.0", "0.2.0"), Some(false));
        assert_eq!(is_newer("0.1.9", "0.2.0"), Some(false));
        assert_eq!(is_newer("v1.0.0", "0.99.99"), Some(true));
        assert_eq!(is_newer("1.0.0-rc.1", "0.9.0"), None);
    }
}
