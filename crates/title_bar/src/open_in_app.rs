use std::path::Path;
use std::sync::Arc;

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use ui::IconName;

/// Key used to persist the most-recently-used external app in the key-value store.
pub const LAST_USED_APP_KEY: &str = "title_bar_open_in_app_last_used";

/// The set of characters that are percent-encoded when embedding a value in a
/// URL. Mirrors `encodeURIComponent`: everything outside the unreserved set is
/// escaped (notably `/` is escaped too, so filesystem paths survive intact).
const URL_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'|')
    .add(b'^')
    .add(b'\\')
    .add(b'[')
    .add(b']')
    .add(b'&');

/// Everything the external apps might need to open the current repository.
pub struct OpenTarget {
    /// On-disk root of the active worktree.
    pub abs_path: Arc<Path>,
    /// Canonical web URL of the repo's default remote, e.g.
    /// `https://github.com/owner/repo`. `None` when there is no recognized
    /// hosting remote.
    pub repo_web_url: Option<String>,
    /// Currently checked-out branch, if any.
    pub branch: Option<String>,
}

/// An external application the current repository can be opened in from the
/// title bar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExternalApp {
    GitHubDesktop,
    Warp,
}

impl ExternalApp {
    /// All supported apps, in the order they appear in the dropdown.
    pub const ALL: [ExternalApp; 2] = [ExternalApp::GitHubDesktop, ExternalApp::Warp];

    /// The default app used when no preference has been persisted yet.
    pub const DEFAULT: ExternalApp = ExternalApp::GitHubDesktop;

    pub fn label(self) -> &'static str {
        match self {
            ExternalApp::GitHubDesktop => "GitHub Desktop",
            ExternalApp::Warp => "Warp",
        }
    }

    pub fn icon(self) -> IconName {
        match self {
            ExternalApp::GitHubDesktop => IconName::GithubDesktop,
            ExternalApp::Warp => IconName::Warp,
        }
    }

    /// Stable identifier used when persisting the preference.
    pub fn id(self) -> &'static str {
        match self {
            ExternalApp::GitHubDesktop => "github_desktop",
            ExternalApp::Warp => "warp",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        ExternalApp::ALL.into_iter().find(|app| app.id() == id)
    }

    /// Builds the URL-scheme link that opens `target` in this app, or `None`
    /// when this app can't act on the target (e.g. GitHub Desktop with no
    /// recognized remote).
    pub fn url(self, target: &OpenTarget) -> Option<String> {
        match self {
            // GitHub Desktop dropped `openLocalRepo`; the supported action is
            // `openRepo/<remote-web-url>?branch=<branch>`, which matches the URL
            // against a repository already known to GitHub Desktop and (if the
            // repo has the branch) checks it out. The remote URL is placed in
            // the path verbatim, exactly like GitHub.com's "Open in Desktop"
            // links; only the branch query value is percent-encoded.
            ExternalApp::GitHubDesktop => {
                let repo_url = target.repo_web_url.as_deref()?;
                let mut url = format!("x-github-client://openRepo/{repo_url}");
                if let Some(branch) = target.branch.as_deref() {
                    let branch = utf8_percent_encode(branch, URL_ENCODE_SET);
                    url.push_str(&format!("?branch={branch}"));
                }
                Some(url)
            }
            // `new_tab` opens a tab in the existing Warp window (falling back to
            // a new window only when none is open), rather than always spawning
            // a new window like `new_window` does.
            ExternalApp::Warp => {
                let path = target.abs_path.to_string_lossy();
                let encoded = utf8_percent_encode(&path, URL_ENCODE_SET);
                Some(format!("warp://action/new_tab?path={encoded}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> OpenTarget {
        OpenTarget {
            abs_path: Arc::from(Path::new("/Users/me/my repo")),
            repo_web_url: Some("https://github.com/owner/repo".to_string()),
            branch: Some("feature/x".to_string()),
        }
    }

    #[test]
    fn round_trips_ids() {
        for app in ExternalApp::ALL {
            assert_eq!(ExternalApp::from_id(app.id()), Some(app));
        }
        assert_eq!(ExternalApp::from_id("unknown"), None);
    }

    #[test]
    fn github_desktop_uses_remote_url_and_branch() {
        assert_eq!(
            ExternalApp::GitHubDesktop.url(&target()),
            Some(
                "x-github-client://openRepo/https://github.com/owner/repo?branch=feature%2Fx"
                    .into()
            )
        );
    }

    #[test]
    fn github_desktop_requires_a_remote() {
        let mut target = target();
        target.repo_web_url = None;
        assert_eq!(ExternalApp::GitHubDesktop.url(&target), None);
    }

    #[test]
    fn warp_opens_a_tab_at_the_encoded_path() {
        assert_eq!(
            ExternalApp::Warp.url(&target()),
            Some("warp://action/new_tab?path=%2FUsers%2Fme%2Fmy%20repo".into())
        );
    }
}
