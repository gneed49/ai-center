//! Local identity parsing. A pasted URL is never an HTTP destination.
use super::models::ToolProvider;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinearIssueKey {
    Id(Uuid),
    Identifier(String),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceLocator {
    NotionPage {
        id: Uuid,
    },
    LinearIssue {
        key: LinearIssueKey,
        expected_workspace_slug: Option<String>,
    },
}
impl SourceLocator {
    #[must_use]
    pub const fn provider(&self) -> ToolProvider {
        match self {
            Self::NotionPage { .. } => ToolProvider::Notion,
            Self::LinearIssue { .. } => ToolProvider::Linear,
        }
    }
    #[must_use]
    pub fn canonical_input(&self) -> String {
        match self {
            Self::NotionPage { id } => id.to_string(),
            Self::LinearIssue {
                key,
                expected_workspace_slug,
            } => {
                let key = match key {
                    LinearIssueKey::Id(id) => id.to_string(),
                    LinearIssueKey::Identifier(key) => key.clone(),
                };
                expected_workspace_slug.as_ref().map_or_else(
                    || key.clone(),
                    |scope| format!("https://linear.app/{scope}/issue/{key}"),
                )
            }
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocatorError;
impl std::fmt::Display for LocatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Copiez un identifiant ou un lien officiel Notion/Linear pris en charge.")
    }
}
impl std::error::Error for LocatorError {}

fn canonical_id(input: &str) -> Option<Uuid> {
    if !matches!(input.len(), 32 | 36) {
        return None;
    }
    Uuid::parse_str(input).ok().filter(|id| !id.is_nil())
}
pub(super) fn linear_identifier(input: &str) -> Option<String> {
    let (prefix, number) = input.rsplit_once('-')?;
    if !(1..=32).contains(&prefix.len())
        || !(1..=18).contains(&number.len())
        || !prefix.bytes().all(|c| c.is_ascii_alphanumeric())
        || !prefix.as_bytes()[0].is_ascii_alphabetic()
        || !number.bytes().all(|c| c.is_ascii_digit())
        || number.starts_with('0')
    {
        return None;
    }
    Some(format!("{}-{number}", prefix.to_ascii_uppercase()))
}
fn official_url(input: &str) -> Result<Url, LocatorError> {
    let url = Url::parse(input).map_err(|_| LocatorError)?;
    // Reject explicit default ports too: URL normalization otherwise hides :443.
    let authority = input
        .split_once("://")
        .ok_or(LocatorError)?
        .1
        .split('/')
        .next()
        .ok_or(LocatorError)?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || authority.contains(':')
        || authority.contains('@')
        || input.contains('\\')
        || url.fragment().is_some()
        || input.split('?').next().is_some_and(|path| {
            path.contains('%') || path.split('/').any(|part| matches!(part, "." | ".."))
        })
    {
        return Err(LocatorError);
    }
    Ok(url)
}
pub(super) fn linear_url(input: &str) -> Result<(String, String, String), LocatorError> {
    let url = official_url(input)?;
    if url.host_str() != Some("linear.app") || url.query().is_some() {
        return Err(LocatorError);
    }
    let parts: Vec<_> = url
        .path()
        .trim_end_matches('/')
        .split('/')
        .skip(1)
        .collect();
    if !(3..=4).contains(&parts.len())
        || parts[1] != "issue"
        || parts[0].is_empty()
        || parts[0].len() > 100
        || !parts[0]
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        || parts.get(3).is_some_and(|slug| {
            slug.is_empty()
                || !slug
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        })
    {
        return Err(LocatorError);
    }
    let identifier = linear_identifier(parts[2]).ok_or(LocatorError)?;
    let workspace = parts[0].to_ascii_lowercase();
    Ok((
        workspace.clone(),
        identifier.clone(),
        format!("https://linear.app/{workspace}/issue/{identifier}"),
    ))
}
pub fn parse_locator(provider: ToolProvider, source: &str) -> Result<SourceLocator, LocatorError> {
    let source = source.trim();
    if source.is_empty() || source.len() > 2048 || source.chars().any(char::is_control) {
        return Err(LocatorError);
    }
    if let Some(id) = canonical_id(source) {
        return Ok(match provider {
            ToolProvider::Notion => SourceLocator::NotionPage { id },
            ToolProvider::Linear => SourceLocator::LinearIssue {
                key: LinearIssueKey::Id(id),
                expected_workspace_slug: None,
            },
        });
    }
    if provider == ToolProvider::Linear {
        if let Some(identifier) = linear_identifier(source) {
            return Ok(SourceLocator::LinearIssue {
                key: LinearIssueKey::Identifier(identifier),
                expected_workspace_slug: None,
            });
        }
        let (workspace, identifier, _) = linear_url(source)?;
        return Ok(SourceLocator::LinearIssue {
            key: LinearIssueKey::Identifier(identifier),
            expected_workspace_slug: Some(workspace),
        });
    }
    let url = official_url(source)?;
    if !matches!(
        url.host_str(),
        Some("notion.so" | "www.notion.so" | "app.notion.com")
    ) {
        return Err(LocatorError);
    }
    // Known share parameters never alter the selected page identity.
    if url
        .query_pairs()
        .any(|(key, value)| !matches!(key.as_ref(), "pvs" | "source") || value.len() > 64)
    {
        return Err(LocatorError);
    }
    let segments: Vec<_> = url
        .path()
        .trim_end_matches('/')
        .split('/')
        .skip(1)
        .collect();
    if segments.is_empty() || segments.len() > 2 || segments.iter().any(|s| s.is_empty()) {
        return Err(LocatorError);
    }
    let last = segments.last().ok_or(LocatorError)?;
    let id = canonical_id(last)
        .or_else(|| {
            // Notion's slug form terminates in an unhyphenated page UUID.
            let (slug, tail) = last.rsplit_once('-')?;
            (!slug.is_empty() && tail.len() == 32 && !slug.contains('%'))
                .then(|| canonical_id(tail))
                .flatten()
        })
        .ok_or(LocatorError)?;
    // Scan every UUID-looking segment, including a second UUID hidden in a slug.
    // An ambiguous pasted path is rejected instead of choosing a convenient suffix.
    let mut identities = std::collections::BTreeSet::new();
    for width in [32, 36] {
        for bytes in url.path().as_bytes().windows(width) {
            if let Ok(candidate) = std::str::from_utf8(bytes)
                && let Some(candidate) = canonical_id(candidate)
            {
                identities.insert(candidate);
            }
        }
    }
    if identities.len() != 1 || !identities.contains(&id) {
        return Err(LocatorError);
    }
    Ok(SourceLocator::NotionPage { id })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_identity_uses_only_official_non_ambiguous_inputs() {
        let id = "abcdef00-0000-4000-8000-000000000001";
        let expected = parse_locator(ToolProvider::Notion, id).unwrap();
        for input in [
            format!("https://www.notion.so/{id}"),
            "https://notion.so/team/Title-abcdef00000040008000000000000001?pvs=4".into(),
        ] {
            assert_eq!(
                parse_locator(ToolProvider::Notion, &input),
                Ok(expected.clone())
            );
        }
        let linear = parse_locator(
            ToolProvider::Linear,
            "https://linear.app/fictif/issue/PROD-42/title",
        )
        .unwrap();
        assert_eq!(
            linear.canonical_input(),
            "https://linear.app/fictif/issue/PROD-42"
        );
        assert_eq!(
            parse_locator(ToolProvider::Linear, "prod-42")
                .unwrap()
                .canonical_input(),
            "PROD-42"
        );
    }
    #[test]
    fn hostile_or_unsupported_inputs_are_refused_before_network() {
        for input in [
            "https://notion.so.evil.example/abcdef00000040008000000000000001",
            "http://notion.so/abcdef00000040008000000000000001",
            "https://notion.site/abcdef00000040008000000000000001",
            "https://notion.so:443/abcdef00000040008000000000000001",
            "https://user@notion.so/abcdef00000040008000000000000001",
            "https://notion.so/abcdef00000040008000000000000001#another",
            "https://notion.so/abcdef00000040008000000000000001/abcdef00000040008000000000000002",
            "https://notion.so/Name-abcdef00000040008000000000000001-abcdef00000040008000000000000002",
            "https://notion.so/%2e%2e/abcdef00000040008000000000000001",
            "00000000-0000-0000-0000-000000000000",
        ] {
            assert_eq!(
                parse_locator(ToolProvider::Notion, input),
                Err(LocatorError)
            );
        }
        for input in [
            "https://linear.app.evil.example/a/issue/PROD-42",
            "https://linear.app/a/issue/PROD-42?token=secret",
            "https://linear.app/a/issue/PROD-42/more/path",
            "PROD-0",
            "PROD-01",
            "PROD-42 mutation",
            "https://linear.app:443/a/issue/PROD-42",
        ] {
            assert_eq!(
                parse_locator(ToolProvider::Linear, input),
                Err(LocatorError)
            );
        }
    }
}
