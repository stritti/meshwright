//! Plugin versions, and the update check every import runs: is a newer release out than the
//! build that is importing?
//!
//! The plugin sandbox has no network, so the component cannot ask GitHub while an import runs.
//! The newest release is therefore baked into the component when it is packaged (`cargo xtask
//! dist` builds with `REVMESH_LATEST_VERSION` set to the latest GitHub release), and each import
//! compares that with the version it was built as ([`update_notice`]).

use std::cmp::Ordering;
use std::fmt;

use crate::i18n::Locale;
use crate::report::{LossItem, Severity};

/// A release version such as `0.1.0-beta.5`, ordered by the semver 2.0.0 rules: numerically by
/// `major.minor.patch`, a pre-release before the release it belongs to, pre-release identifiers
/// numerically when both are numeric and lower than text, otherwise in ASCII order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    pre_release: Vec<PreRelease>,
}

/// One dot-separated identifier of a pre-release (`beta.5` is `Text("beta")`, `Numeric(5)`).
#[derive(Debug, Clone, PartialEq, Eq)]
enum PreRelease {
    Numeric(u64),
    Text(String),
}

impl Version {
    /// Parses `major.minor.patch` with an optional `-pre.release` and an optional `+build`
    /// (ignored), a leading `v` as release tags carry it included. `None` when the text is not
    /// a version.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.strip_prefix('v').unwrap_or(text).split('+').next()?;
        let (core, pre) = match text.split_once('-') {
            Some((_, "")) => return None,
            Some((core, pre)) => (core, pre),
            None => (text, ""),
        };
        let mut parts = core.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        let pre_release = if pre.is_empty() { Vec::new() } else { pre.split('.').map(pre_release_id).collect::<Option<Vec<_>>>()? };
        Some(Self { major, minor, patch, pre_release })
    }
}

/// One pre-release identifier: numeric when it is all digits, text otherwise; `None` when it is
/// empty or holds characters semver does not allow.
fn pre_release_id(id: &str) -> Option<PreRelease> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    if id.bytes().all(|b| b.is_ascii_digit()) { id.parse().ok().map(PreRelease::Numeric) } else { Some(PreRelease::Text(id.to_owned())) }
}

/// Orders two pre-release identifier lists by the semver rules; an empty list is the release
/// itself and sorts after every pre-release.
fn cmp_pre_release(a: &[PreRelease], b: &[PreRelease]) -> Ordering {
    match (a.is_empty(), b.is_empty()) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        (false, false) => {}
    }
    for (x, y) in a.iter().zip(b.iter()) {
        let ordering = match (x, y) {
            (PreRelease::Numeric(n), PreRelease::Numeric(m)) => n.cmp(m),
            (PreRelease::Numeric(_), PreRelease::Text(_)) => Ordering::Less,
            (PreRelease::Text(_), PreRelease::Numeric(_)) => Ordering::Greater,
            (PreRelease::Text(s), PreRelease::Text(t)) => s.cmp(t),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    a.len().cmp(&b.len())
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| cmp_pre_release(&self.pre_release, &other.pre_release))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre_release.is_empty() {
            write!(f, "-")?;
            for (i, id) in self.pre_release.iter().enumerate() {
                if i > 0 {
                    write!(f, ".")?;
                }
                match id {
                    PreRelease::Numeric(n) => write!(f, "{n}")?,
                    PreRelease::Text(s) => write!(f, "{s}")?,
                }
            }
        }
        Ok(())
    }
}

/// The report item an import adds when `latest` names a newer release than `current`, else
/// `None` — also when either text is not a version, so a broken baked-in value stays silent.
#[must_use]
pub fn update_notice(current: &str, latest: &str, locale: Locale) -> Option<LossItem> {
    let current = Version::parse(current)?;
    let latest = Version::parse(latest)?;
    if latest <= current {
        return None;
    }
    let current_text = current.to_string();
    let latest_text = latest.to_string();
    Some(LossItem {
        severity: Severity::Info,
        title: locale.text("report.update.title").into(),
        subject: "revmesh".into(),
        message: locale.format("report.update.message", &[("latest", &latest_text), ("current", &current_text)]),
        code: "revmesh.update_available",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap_or_else(|| panic!("{text} parses"))
    }

    #[test]
    fn parses_and_prints_versions() {
        assert_eq!(version("0.1.0-beta.5").to_string(), "0.1.0-beta.5");
        assert_eq!(version("v1.2.3").to_string(), "1.2.3");
        assert_eq!(version("1.0.0+build.7").to_string(), "1.0.0");
        assert_eq!(version("0.1.0-beta.5").to_string(), "0.1.0-beta.5");
        assert_eq!(Version::parse(""), None);
        assert_eq!(Version::parse("1.0"), None);
        assert_eq!(Version::parse("1.0.0.0"), None);
        assert_eq!(Version::parse("a.b.c"), None);
        assert_eq!(Version::parse("1.0.0-"), None);
        assert_eq!(Version::parse("1.0.0-beta..5"), None);
        assert_eq!(Version::parse("1.0.0-be ta"), None);
    }

    #[test]
    fn orders_by_semver() {
        assert!(version("0.1.0-beta.5") < version("0.1.0-beta.6"));
        assert!(version("0.1.0-beta.9") < version("0.1.0-beta.10"));
        assert!(version("0.1.0-beta.5") < version("0.1.0"));
        assert!(version("0.1.0") < version("0.2.0"));
        assert!(version("0.9.9") < version("1.0.0"));
        assert!(version("1.0.0-1") < version("1.0.0-alpha"));
        assert!(version("1.0.0-alpha") < version("1.0.0-alpha.1"));
        assert!(version("1.0.0-alpha.1") < version("1.0.0-alpha.beta"));
        assert!(version("1.0.0-beta") < version("1.0.0-beta.2"));
        assert!(version("1.0.0-beta.2") < version("1.0.0-beta.11"));
        assert!(version("1.0.0-beta.11") < version("1.0.0-rc.1"));
        assert!(version("1.0.0-rc.1") < version("1.0.0"));
        assert_eq!(version("0.1.0-beta.5"), version("v0.1.0-beta.5"));
    }

    #[test]
    fn a_newer_release_gives_a_notice() {
        let notice = update_notice("0.1.0-beta.5", "0.2.0", Locale::En).expect("0.2.0 is newer");
        assert_eq!(notice.severity, Severity::Info);
        assert_eq!(notice.code, "revmesh.update_available");
        assert_eq!(notice.subject, "revmesh");
        assert_eq!(notice.title, "A newer version of revmesh is available");
        assert!(notice.message.contains("0.2.0"), "{}", notice.message);
        assert!(notice.message.contains("0.1.0-beta.5"), "{}", notice.message);
        assert!(!notice.message.contains("{latest}"), "{}", notice.message);
    }

    #[test]
    fn the_notice_is_in_the_imports_language() {
        let notice = update_notice("0.1.0-beta.5", "0.1.0-beta.6", Locale::De).expect("beta.6 is newer than beta.5");
        assert_eq!(notice.title, "Eine neuere Version von revmesh ist verfügbar");
        assert!(notice.message.contains("0.1.0-beta.6"), "{}", notice.message);
        assert!(notice.message.contains("0.1.0-beta.5"), "{}", notice.message);
    }

    #[test]
    fn no_notice_without_a_newer_release() {
        assert_eq!(update_notice("0.1.0-beta.5", "0.1.0-beta.5", Locale::En), None);
        assert_eq!(update_notice("0.2.0", "0.1.0-beta.5", Locale::En), None);
        assert_eq!(update_notice("0.1.0", "0.1.0-beta.5", Locale::En), None, "a pre-release is older than its release");
        assert_eq!(update_notice("0.1.0-beta.5", "not a version", Locale::En), None);
        assert_eq!(update_notice("", "", Locale::En), None);
    }
}
