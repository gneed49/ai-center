//! Conservative import guard, not a guarantee that source code is secret-free.
//! Never return matching text: an excluded observation contains only a reason.

pub(super) fn sensitive_path(path: &str) -> bool {
    path.split('/').any(|part| {
        let part = part.to_ascii_lowercase();
        part == ".env"
            || part.starts_with(".env.")
            || part == ".ssh"
            || part == ".aws"
            || part == ".npmrc"
            || part == ".netrc"
            || part.starts_with("id_rsa")
            || part.starts_with("id_ed25519")
            || part.starts_with("id_ecdsa")
            || part == "credentials.json"
            || part == "service-account.json"
            || part == "secrets.json"
            || [".pem", ".key", ".p12", ".pfx", ".keystore"]
                .iter()
                .any(|suffix| part.ends_with(suffix))
    })
}

pub(super) fn contains_likely_credential(text: &str) -> bool {
    if text.contains("PRIVATE KEY-----") {
        return true;
    }
    let tokens = text.split(|c: char| !c.is_ascii_alphanumeric() && !matches!(c, '-' | '_'));
    if tokens.into_iter().any(|token| {
        token.len() >= 20
            && [
                "sk-",
                "github_pat_",
                "ghp_",
                "gho_",
                "ghu_",
                "ghs_",
                "ghr_",
                "lin_api_",
                "ntn_",
                "xoxb-",
                "xoxp-",
            ]
            .iter()
            .any(|prefix| token.starts_with(prefix))
    }) {
        return true;
    }
    text.lines().any(sensitive_assignment)
        || text.split_whitespace().any(|token| {
            let candidate =
                token.trim_matches(|c: char| matches!(c, '\'' | '"' | '(' | ')' | ',' | ';'));
            url::Url::parse(candidate)
                .is_ok_and(|url| url.password().is_some_and(|value| !value.is_empty()))
        })
}

fn sensitive_assignment(line: &str) -> bool {
    let Some(separator) = line.find(['=', ':']) else {
        return false;
    };
    let (key, value) = (&line[..separator], &line[separator + 1..]);
    let key = key
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase();
    if ![
        "apikey",
        "apisecret",
        "accesstoken",
        "authtoken",
        "clientsecret",
        "password",
        "privatekey",
        "servicekey",
    ]
    .iter()
    .any(|name| key.contains(name))
    {
        return false;
    }
    let value = value.trim();
    // Ignore runtime lookups; inspect quoted literals and plain config values.
    let value = if let Some(quote) = value.chars().next().filter(|c| matches!(c, '\'' | '"')) {
        value[1..].split(quote).next().unwrap_or_default()
    } else {
        value.trim_end_matches([',', ';']).trim()
    };
    value.len() >= 8
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | '+' | '=' | '!'))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excludes_secret_paths_before_remote_read() {
        for path in [
            ".env",
            "config/.env.production",
            ".ssh/id_ed25519",
            "config/key.PEM",
            ".aws/credentials",
            "config/service-account.json",
        ] {
            assert!(sensitive_path(path));
        }
        for path in ["src/environment.rs", "README.md", "src/crypto.rs"] {
            assert!(!sensitive_path(path));
        }
    }
    #[test]
    fn credential_shapes_are_blocked_but_runtime_lookups_remain_readable() {
        let synthetic = format!("const TOKEN = \"ghp_{}\";", "FICTIF".repeat(6));
        assert!(contains_likely_credential(&synthetic));
        let synthetic_key = format!(
            "-----BEGIN {}-----\n[FICTIF]\n-----END {}-----",
            "PRIVATE KEY", "PRIVATE KEY"
        );
        assert!(contains_likely_credential(&synthetic_key));
        assert!(contains_likely_credential(
            "api_key: \"FICTIF-cle-test-seulement\""
        ));
        for encoded_assignment in [
            "api_key: \"RklDVElGX09OTFlfREVNTw==\"",
            "{\"api_key\": \"RklDVElGX09OTFlfREVNTw==\"}",
        ] {
            assert!(contains_likely_credential(encoded_assignment));
        }
        assert!(contains_likely_credential(
            "postgresql://fictif:synthetic@localhost/db"
        ));
        assert!(!contains_likely_credential(
            "let api_key = std::env::var(\"OPENAI_API_KEY\")?;"
        ));
        assert!(!contains_likely_credential(
            "const LIMIT: usize = 120;\nfn expires() -> bool { false }"
        ));
    }
}
