use std::{sync::Arc, time::Duration};

use axum::http::{HeaderMap, header};
use chrono::{DateTime, TimeDelta, Utc};
use jsonwebtoken::{
    Algorithm, DecodingKey, Validation, decode, decode_header,
    jwk::{Jwk, JwkSet},
};
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    config::AuthMode,
    error::{AppError, AppResult},
};

const WORKSPACE_HEADER: &str = "x-ai-center-workspace-id";

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub actor_id: Uuid,
    pub workspace_id: Uuid,
    pub workspace_internal_id: Option<i64>,
    pub workspace_role: String,
}

#[derive(Clone)]
pub struct AuthRuntime {
    mode: AuthMode,
    local_context: RequestContext,
    verifier: Option<Arc<SupabaseJwtVerifier>>,
    company_creators: std::collections::HashSet<Uuid>,
}

impl AuthRuntime {
    /// Builds the authentication runtime and its hardened JWKS client.
    ///
    /// # Errors
    ///
    /// Returns an internal configuration error when the HTTP client cannot be
    /// built; authentication never falls back to a weaker client policy.
    pub fn new(
        mode: AuthMode,
        local_workspace_id: Uuid,
        local_actor_id: Uuid,
        supabase_url: Option<String>,
    ) -> AppResult<Self> {
        let verifier = supabase_url
            .map(|url| SupabaseJwtVerifier::new(&url).map(Arc::new))
            .transpose()?;
        let company_creators = parse_company_creators(
            &std::env::var("AI_CENTER_COMPANY_CREATORS").unwrap_or_default(),
        )?;
        Ok(Self {
            mode,
            company_creators,
            local_context: RequestContext {
                actor_id: local_actor_id,
                workspace_id: local_workspace_id,
                workspace_internal_id: None,
                workspace_role: "owner".into(),
            },
            verifier,
        })
    }

    /// Restricts private-instance bootstrap to approved creators and owners.
    /// # Errors
    /// Returns a database error when existing ownership cannot be verified.
    pub async fn may_create_company(&self, actor: Uuid, pool: &PgPool) -> AppResult<bool> {
        if actor.is_nil() {
            return Ok(false);
        }
        if self.mode == AuthMode::Local || self.company_creators.contains(&actor) {
            return Ok(true);
        }
        let mut tx = pool.begin().await?;
        sqlx::query("select set_config('app.current_actor_id',$1,true)")
            .bind(actor.to_string())
            .execute(&mut *tx)
            .await?;
        let owner = sqlx::query_scalar(
            "select exists(select 1 from app.list_actor_workspaces() where role='owner')",
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(owner)
    }

    /// Authenticates a request and resolves its accepted workspace membership.
    ///
    /// # Errors
    ///
    /// Returns an authorization or database error when the request cannot be
    /// associated with an accepted workspace membership.
    pub async fn authenticate(
        &self,
        headers: &HeaderMap,
        pool: &PgPool,
    ) -> AppResult<RequestContext> {
        if self.mode == AuthMode::Local {
            // The runtime role is FORCE-RLS and has no bootstrap access to the
            // workspace table. Resolve the configured local identity through
            // the same narrow SECURITY DEFINER membership function as real
            // authentication; local mode must not become an RLS bypass.
            let membership: Option<(i64, String)> = sqlx::query_as(
                "select workspace_id, role
                 from app.authorize_workspace_member($1, $2)",
            )
            .bind(self.local_context.workspace_id)
            .bind(self.local_context.actor_id)
            .fetch_optional(pool)
            .await?;
            let (workspace_internal_id, workspace_role) = membership.ok_or(AppError::Forbidden)?;
            let mut context = self.local_context.clone();
            context.workspace_internal_id = Some(workspace_internal_id);
            context.workspace_role = workspace_role;
            return Ok(context);
        }

        let token = headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .filter(|value| !value.is_empty())
            .ok_or(AppError::Unauthorized)?;
        let workspace_id = headers
            .get(WORKSPACE_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<Uuid>().ok())
            .ok_or_else(|| AppError::Invalid("X-AI-Center-Workspace-Id is required".into()))?;
        let verifier = self
            .verifier
            .as_ref()
            .ok_or_else(|| AppError::Internal("Supabase JWT verifier is not configured".into()))?;
        let actor_id = verifier.verify(token).await?;
        let membership: Option<(i64, String)> = sqlx::query_as(
            "select workspace_id, role
             from app.authorize_workspace_member($1, $2)",
        )
        .bind(workspace_id)
        .bind(actor_id)
        .fetch_optional(pool)
        .await?;
        let (workspace_internal_id, workspace_role) = membership.ok_or(AppError::Forbidden)?;
        Ok(RequestContext {
            actor_id,
            workspace_id,
            workspace_internal_id: Some(workspace_internal_id),
            workspace_role,
        })
    }

    /// Verifies identity without selecting a workspace. This is used only by
    /// `GET /api/workspaces`, before the client can send a workspace header.
    ///
    /// # Errors
    ///
    /// Returns `Unauthorized` when the bearer token is missing or invalid.
    pub async fn authenticate_actor(&self, headers: &HeaderMap) -> AppResult<Uuid> {
        if self.mode == AuthMode::Local {
            return Ok(self.local_context.actor_id);
        }
        let token = headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .filter(|value| !value.is_empty())
            .ok_or(AppError::Unauthorized)?;
        self.verifier
            .as_ref()
            .ok_or_else(|| AppError::Internal("Supabase JWT verifier is not configured".into()))?
            .verify(token)
            .await
    }
}

fn parse_company_creators(value: &str) -> AppResult<std::collections::HashSet<Uuid>> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse::<Uuid>()
                .ok()
                .filter(|actor| !actor.is_nil())
                .ok_or_else(|| {
                    AppError::Internal(
                        "AI_CENTER_COMPANY_CREATORS must contain nonempty actor UUIDs".into(),
                    )
                })
        })
        .collect()
}

struct CachedJwks {
    set: JwkSet,
    fetched_at: DateTime<Utc>,
}

pub struct SupabaseJwtVerifier {
    client: Client,
    issuer: String,
    jwks_url: String,
    cache: RwLock<Option<CachedJwks>>,
}

#[derive(Debug, Deserialize)]
struct SupabaseClaims {
    sub: String,
    #[allow(dead_code)]
    exp: usize,
    #[allow(dead_code)]
    iss: String,
    #[allow(dead_code)]
    aud: Audience,
    role: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}

impl Audience {
    fn contains_authenticated(&self) -> bool {
        match self {
            Self::One(value) => value == "authenticated",
            Self::Many(values) => values.iter().any(|value| value == "authenticated"),
        }
    }
}

impl SupabaseJwtVerifier {
    fn new(supabase_url: &str) -> AppResult<Self> {
        let base = supabase_url.trim_end_matches('/');
        Ok(Self {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(10))
                .build()
                .map_err(|_| AppError::Internal("failed to build Supabase JWKS client".into()))?,
            issuer: format!("{base}/auth/v1"),
            jwks_url: format!("{base}/auth/v1/.well-known/jwks.json"),
            cache: RwLock::new(None),
        })
    }

    /// Verifies signature, issuer, audience, expiry, role, and subject.
    ///
    /// # Errors
    ///
    /// Returns `Unauthorized` when the token or the current JWKS is invalid.
    pub async fn verify(&self, token: &str) -> AppResult<Uuid> {
        let header = decode_header(token).map_err(|_| AppError::Unauthorized)?;
        if !matches!(
            header.alg,
            Algorithm::RS256 | Algorithm::ES256 | Algorithm::EdDSA
        ) {
            return Err(AppError::Unauthorized);
        }
        let kid = header.kid.as_deref().ok_or(AppError::Unauthorized)?;
        let mut jwk = self.find_key(kid, false).await?;
        if jwk.is_none() {
            jwk = self.find_key(kid, true).await?;
        }
        let jwk = jwk.ok_or(AppError::Unauthorized)?;
        let key = DecodingKey::from_jwk(&jwk).map_err(|_| AppError::Unauthorized)?;
        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&["authenticated"]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        validation.leeway = 30;
        let claims = decode::<SupabaseClaims>(token, &key, &validation)
            .map_err(|_| AppError::Unauthorized)?
            .claims;
        if claims.role != "authenticated" || !claims.aud.contains_authenticated() {
            return Err(AppError::Unauthorized);
        }
        claims.sub.parse().map_err(|_| AppError::Unauthorized)
    }

    async fn find_key(&self, kid: &str, force_refresh: bool) -> AppResult<Option<Jwk>> {
        if !force_refresh {
            let cache = self.cache.read().await;
            if let Some(cache) = cache.as_ref()
                && cache.fetched_at > Utc::now() - TimeDelta::minutes(10)
            {
                return Ok(cache.set.find(kid).cloned());
            }
        }
        let response = self
            .client
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|_| AppError::Unauthorized)?;
        if !response.status().is_success() {
            return Err(AppError::Unauthorized);
        }
        let set: JwkSet = response.json().await.map_err(|_| AppError::Unauthorized)?;
        let found = set.find(kid).cloned();
        *self.cache.write().await = Some(CachedJwks {
            set,
            fetched_at: Utc::now(),
        });
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{DecodingKey, crypto};

    #[test]
    fn private_bootstrap_allowlist_rejects_malformed_and_nil_identities() {
        assert!(
            parse_company_creators("")
                .expect("empty allowlist")
                .is_empty()
        );
        let actor = Uuid::new_v4();
        let allowed =
            parse_company_creators(&format!(" {actor}, {actor} ")).expect("UUID allowlist");
        assert_eq!(allowed.len(), 1);
        assert!(allowed.contains(&actor));
        assert!(parse_company_creators("not-an-identity").is_err());
        assert!(parse_company_creators(&Uuid::nil().to_string()).is_err());
    }

    #[test]
    fn audience_shapes_deserialize_without_becoming_authorization_data() {
        let one: Audience = serde_json::from_str("\"authenticated\"").expect("single audience");
        let many: Audience =
            serde_json::from_str("[\"authenticated\"]").expect("multiple audiences");
        assert!(matches!(one, Audience::One(value) if value == "authenticated"));
        assert!(matches!(many, Audience::Many(values) if values.len() == 1));
    }

    #[test]
    fn jwt_crypto_provider_is_available() {
        let verified = crypto::verify(
            "AA",
            b"unsigned-test-message",
            &DecodingKey::from_secret(b"test-only-secret"),
            Algorithm::HS256,
        )
        .expect("the configured JWT crypto provider must be usable");

        assert!(!verified, "an invalid signature must still be rejected");
    }
}
