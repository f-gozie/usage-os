//! Enrichment: turn raw capture signals into stored facts (the architecture's
//! `enrich/` layer). Cross-platform and CI-testable — it consumes the
//! `url`/`cwd`/`title` signals the capture layer gathers and produces a `site`
//! string and a project assignment (D30). Embedding-based context categorization
//! is a later phase.

mod project;

pub use project::{infer_project, ProjectAssignment, ProjectSignals};

/// Parse the host/"site" from a URL (e.g. `https://github.com/x` → `github.com`),
/// stripping the port. Returns `None` for `file://` and unparseable URLs.
pub fn parse_site(url: &str) -> Option<String> {
    let (_scheme, host, _path) = project::parse_url(url);
    let host = host.split(':').next().unwrap_or("");
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_site_extracts_host_without_port() {
        assert_eq!(
            parse_site("https://github.com/a/b").as_deref(),
            Some("github.com")
        );
        assert_eq!(
            parse_site("http://localhost:3002/x").as_deref(),
            Some("localhost")
        );
        assert_eq!(
            parse_site("https://eu.posthog.com/p/1").as_deref(),
            Some("eu.posthog.com")
        );
    }

    #[test]
    fn parse_site_none_for_file_and_garbage() {
        assert_eq!(parse_site("file:///Users/x/a.html"), None);
        assert_eq!(parse_site("not a url"), None);
    }
}
