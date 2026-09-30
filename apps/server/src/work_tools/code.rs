//! Selected immutable Git objects. Files are read as data, never executed.
use super::{audit, client::ToolClient, credential, reliability};
use crate::{
    artifacts::{complete, editor, project_id},
    error::{AppError, AppResult},
    idempotency::IdempotencyLease,
    service::AppState,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, Utc};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    time::Duration,
};
use uuid::Uuid;

const FILE_LIMIT: usize = 65_536;
const CORPUS_LIMIT: usize = 262_144;
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadCode {
    pub connection_id: Uuid,
    pub repository: String,
    pub commit_sha: String,
    pub paths: Vec<String>,
}
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Corpus {
    pub public_id: Uuid,
    pub project_id: Uuid,
    pub connection_id: Uuid,
    pub repository: String,
    pub commit_sha: String,
    pub commit_verified: bool,
    pub requested_paths: Value,
    pub observed_at: DateTime<Utc>,
}
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct FileObservation {
    pub public_id: Uuid,
    pub path: String,
    pub status: String,
    pub reason_code: Option<String>,
    pub blob_sha: Option<String>,
    pub content_hash: Option<String>,
    pub content_text: Option<String>,
    pub line_count: i32,
    pub observed_at: DateTime<Utc>,
}
#[derive(Serialize, Deserialize)]
pub struct CodeDetail {
    pub corpus: Corpus,
    pub files: Vec<FileObservation>,
}
#[derive(Serialize)]
pub struct CodeList {
    pub items: Vec<Corpus>,
    pub limit: i64,
    pub offset: i64,
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CodeQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
#[derive(Debug)]
pub(super) struct ObservedFile {
    path: String,
    status: &'static str,
    reason: Option<&'static str>,
    blob: Option<String>,
    text: Option<String>,
}
impl ObservedFile {
    fn unavailable(path: &str, status: &'static str, reason: &'static str) -> Self {
        Self {
            path: path.into(),
            status,
            reason: Some(reason),
            blob: None,
            text: None,
        }
    }
}
fn valid_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn validate(input: &mut ReadCode) -> AppResult<()> {
    let parts = input.repository.split('/').collect::<Vec<_>>();
    if input.connection_id.is_nil()
        || parts.len() != 2
        || !parts.iter().all(|part| {
            !part.is_empty()
                && part.len() <= 100
                && *part != "."
                && *part != ".."
                && part
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
        })
        || !valid_sha(&input.commit_sha)
        || input.paths.is_empty()
        || input.paths.len() > 10
    {
        return Err(AppError::Invalid(
            "Choose a GitHub repository, full 40-character commit SHA and 1–10 file paths".into(),
        ));
    }
    let mut seen = BTreeSet::new();
    for path in &input.paths {
        if super::code_privacy::sensitive_path(path) {
            return Err(AppError::Invalid("Ce fichier peut contenir des identifiants secrets. Choisissez uniquement des fichiers de code à analyser.".into()));
        }
        if path.len() > 512
            || path.contains('\\')
            || path.chars().any(char::is_control)
            || path.split('/').count() > 8
            || path
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == "..")
            || !seen.insert(path)
        {
            return Err(AppError::Invalid("Use distinct relative file paths, without parent traversal and at most eight segments".into()));
        }
    }
    input.repository.make_ascii_lowercase();
    input.commit_sha.make_ascii_lowercase();
    Ok(())
}
const CORPUS_SELECT: &str = "select c.public_id,p.public_id as project_id,w.public_id as connection_id,c.repository,c.commit_sha,c.commit_verified,c.requested_paths,c.observed_at from app.github_code_corpora c join app.projects p on p.id=c.project_id join app.work_tool_connections w on w.id=c.connection_id";
async fn detail_tx(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> AppResult<CodeDetail> {
    let corpus = sqlx::query_as(&format!(
        "{CORPUS_SELECT} where c.public_id=$1 and c.workspace_id=app.current_workspace_id()"
    ))
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)?;
    let files=sqlx::query_as("select f.public_id,f.path,f.status,f.reason_code,f.blob_sha,f.content_hash,f.content_text,f.line_count,f.observed_at from app.github_code_file_observations f join app.github_code_corpora c on c.id=f.corpus_id where c.public_id=$1 and c.workspace_id=app.current_workspace_id() order by f.path")
        .bind(id).fetch_all(&mut **tx).await?;
    Ok(CodeDetail { corpus, files })
}
pub async fn detail(state: &AppState, id: Uuid) -> AppResult<CodeDetail> {
    let mut tx = state.begin_request().await?;
    let result = detail_tx(&mut tx, id).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn list(state: &AppState, project: Uuid, query: CodeQuery) -> AppResult<CodeList> {
    let limit = query.limit.unwrap_or(25);
    let offset = query.offset.unwrap_or(0);
    if !(1..=100).contains(&limit) || !(0..=1_000_000).contains(&offset) {
        return Err(AppError::Invalid("Invalid pagination".into()));
    }
    let mut tx = state.begin_request().await?;
    let scope = project_id(&mut tx, project).await?;
    let items=sqlx::query_as(&format!("{CORPUS_SELECT} where c.project_id=$1 and c.workspace_id=app.current_workspace_id() order by c.id desc limit $2 offset $3"))
        .bind(scope).bind(limit).bind(offset).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(CodeList {
        items,
        limit,
        offset,
    })
}
pub async fn read(
    state: &AppState,
    project: Uuid,
    input: ReadCode,
    lease: Option<&IdempotencyLease>,
) -> AppResult<CodeDetail> {
    read_using(state, project, input, lease, &ToolClient::official()?).await
}
pub async fn read_using(
    state: &AppState,
    project: Uuid,
    mut input: ReadCode,
    lease: Option<&IdempotencyLease>,
    client: &ToolClient,
) -> AppResult<CodeDetail> {
    editor(state)?;
    validate(&mut input)?;
    let mut tx = state.begin_request().await?;
    let scope = project_id(&mut tx, project).await?;
    crate::company::data::require_active(&mut tx, scope).await?;
    tx.commit().await?;
    let credential = credential(state, input.connection_id).await?;
    if credential.provider != "github" {
        return Err(AppError::Invalid("A GitHub connection is required".into()));
    }
    // One bounded corpus operation counts against the durable read quota.
    reliability::admit_read(state, project).await?;
    let (verified, files) = match tokio::time::timeout(
        Duration::from_secs(60),
        collect(client, &credential.secret, &input),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => (
            false,
            input
                .paths
                .iter()
                .map(|path| ObservedFile::unavailable(path, "unavailable", "corpus_timeout"))
                .collect(),
        ),
    };
    // Recheck credential revocation and project archive before persisting evidence.
    let current = super::credential(state, input.connection_id).await?;
    if current.revision != credential.revision {
        return Err(AppError::Conflict(
            "Connection changed while reading code".into(),
        ));
    }
    let mut tx = state.begin_request().await?;
    crate::company::data::require_active(&mut tx, scope).await?;
    let connection:i64=sqlx::query_scalar("select id from app.work_tool_connections where public_id=$1 and workspace_id=app.current_workspace_id() and enabled")
        .bind(input.connection_id).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    let (corpus,id):(i64,Uuid)=sqlx::query_as("insert into app.github_code_corpora(workspace_id,project_id,connection_id,repository,commit_sha,commit_verified,requested_paths,requested_by_actor_id) values(app.current_workspace_id(),$1,$2,$3,$4,$5,$6,$7) returning id,public_id")
        .bind(scope).bind(connection).bind(&input.repository).bind(&input.commit_sha).bind(verified).bind(json!(input.paths)).bind(state.actor_id).fetch_one(&mut *tx).await?;
    for file in files {
        let hash = file
            .text
            .as_ref()
            .map(|text| format!("{:x}", Sha256::digest(text.as_bytes())));
        let lines = i32::try_from(file.text.as_ref().map_or(0, |text| text.lines().count()))
            .map_err(|_| AppError::Invalid("File has too many lines".into()))?;
        sqlx::query("insert into app.github_code_file_observations(workspace_id,project_id,corpus_id,path,status,reason_code,blob_sha,content_hash,content_text,line_count) values(app.current_workspace_id(),$1,$2,$3,$4,$5,$6,$7,$8,$9)")
            .bind(scope).bind(corpus).bind(file.path).bind(file.status).bind(file.reason).bind(file.blob).bind(hash).bind(file.text).bind(lines).execute(&mut *tx).await?;
    }
    audit(&mut tx,state,"github_code.observed",id,json!({"project_id":project,"repository":input.repository,"commit_sha":input.commit_sha,"commit_verified":verified})).await?;
    let result = detail_tx(&mut tx, id).await?;
    complete(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}
async fn collect(
    client: &ToolClient,
    secret: &SecretString,
    input: &ReadCode,
) -> (bool, Vec<ObservedFile>) {
    let commit = client
        .request(
            "github",
            secret,
            reqwest::Method::GET,
            &format!(
                "/repos/{}/git/commits/{}",
                input.repository, input.commit_sha
            ),
            None,
            false,
        )
        .await;
    let root = match commit {
        Ok(value) if value["sha"] == input.commit_sha => value
            .pointer("/tree/sha")
            .and_then(Value::as_str)
            .filter(|sha| valid_sha(sha))
            .map(str::to_owned),
        _ => None,
    };
    let Some(root) = root else {
        return (
            false,
            input
                .paths
                .iter()
                .map(|path| ObservedFile::unavailable(path, "unavailable", "commit_not_verified"))
                .collect(),
        );
    };
    let mut trees = BTreeMap::new();
    let mut remaining = CORPUS_LIMIT;
    let mut files = Vec::new();
    let mut calls = 1;
    for path in &input.paths {
        let mut file = read_file(
            client,
            secret,
            &input.repository,
            &root,
            path,
            &mut trees,
            &mut calls,
        )
        .await;
        if let Some(text) = &file.text {
            if text.len() > remaining {
                file = ObservedFile::unavailable(path, "too_large", "corpus_size_limit");
            } else {
                remaining -= text.len();
            }
        }
        files.push(file);
    }
    (true, files)
}
async fn read_file(
    client: &ToolClient,
    secret: &SecretString,
    repo: &str,
    root: &str,
    path: &str,
    trees: &mut BTreeMap<String, Value>,
    calls: &mut i32,
) -> ObservedFile {
    let parts = path.split('/').collect::<Vec<_>>();
    let mut tree = root.to_owned();
    for (index, part) in parts.iter().enumerate() {
        if !trees.contains_key(&tree) {
            if *calls >= 32 {
                return ObservedFile::unavailable(path, "unavailable", "request_budget_exhausted");
            }
            *calls += 1;
            let value = match client
                .request(
                    "github",
                    secret,
                    reqwest::Method::GET,
                    &format!("/repos/{repo}/git/trees/{tree}"),
                    None,
                    false,
                )
                .await
            {
                Ok(value) => value,
                Err(error) => return remote_file_error(path, error.code),
            };
            if value["sha"] != tree || value["truncated"] != false || !value["tree"].is_array() {
                return ObservedFile::unavailable(path, "unavailable", "tree_incomplete");
            }
            trees.insert(tree.clone(), value);
        }
        let Some(entry) = trees[&tree]["tree"]
            .as_array()
            .and_then(|entries| entries.iter().find(|entry| entry["path"] == *part))
        else {
            return ObservedFile::unavailable(path, "missing", "path_absent_from_verified_tree");
        };
        let Some(sha) = entry["sha"].as_str().filter(|sha| valid_sha(sha)) else {
            return ObservedFile::unavailable(path, "unavailable", "invalid_git_identity");
        };
        if index + 1 < parts.len() {
            if entry["type"] != "tree" || entry["mode"] != "040000" {
                return ObservedFile::unavailable(path, "unsupported", "not_a_directory");
            }
            tree = sha.into();
            continue;
        }
        if entry["type"] != "blob" || !matches!(entry["mode"].as_str(), Some("100644" | "100755")) {
            return ObservedFile::unavailable(
                path,
                "unsupported",
                "symlink_submodule_or_directory",
            );
        }
        if entry["size"]
            .as_u64()
            .is_none_or(|size| size > FILE_LIMIT as u64)
        {
            return ObservedFile::unavailable(path, "too_large", "file_size_limit");
        }
        if *calls >= 32 {
            return ObservedFile::unavailable(path, "unavailable", "request_budget_exhausted");
        }
        *calls += 1;
        return match client
            .request(
                "github",
                secret,
                reqwest::Method::GET,
                &format!("/repos/{repo}/git/blobs/{sha}"),
                None,
                false,
            )
            .await
        {
            Ok(value) => decode_blob(path, sha, &value),
            Err(error) => remote_file_error(path, error.code),
        };
    }
    ObservedFile::unavailable(path, "unavailable", "invalid_path")
}
fn remote_file_error(path: &str, code: &'static str) -> ObservedFile {
    ObservedFile::unavailable(
        path,
        if matches!(code, "remote_permission" | "remote_authentication") {
            "inaccessible"
        } else {
            "unavailable"
        },
        code,
    )
}
fn decode_blob(path: &str, sha: &str, value: &Value) -> ObservedFile {
    if value["sha"] != sha || value["encoding"] != "base64" {
        return ObservedFile::unavailable(path, "unavailable", "blob_identity_or_encoding_invalid");
    }
    let Some(encoded) = value["content"].as_str() else {
        return ObservedFile::unavailable(path, "unavailable", "blob_content_missing");
    };
    let compact = encoded
        .chars()
        .filter(|ch| !matches!(ch, '\n' | '\r'))
        .collect::<String>();
    if compact.len() > FILE_LIMIT.div_ceil(3) * 4 {
        return ObservedFile::unavailable(path, "too_large", "file_size_limit");
    }
    let Ok(bytes) = STANDARD.decode(compact) else {
        return ObservedFile::unavailable(path, "unavailable", "blob_encoding_invalid");
    };
    if bytes.len() > FILE_LIMIT {
        return ObservedFile::unavailable(path, "too_large", "file_size_limit");
    }
    if value["size"].as_u64() != Some(bytes.len() as u64) || blob_sha(&bytes) != sha {
        return ObservedFile::unavailable(path, "unavailable", "blob_hash_mismatch");
    }
    let text = match String::from_utf8(bytes) {
        Ok(text) if !text.contains('\0') => text,
        _ => return ObservedFile::unavailable(path, "binary", "not_utf8_text"),
    };
    if super::code_privacy::contains_likely_credential(&text) {
        return ObservedFile::unavailable(path, "unsupported", "sensitive_content_excluded");
    }
    ObservedFile {
        path: path.into(),
        status: "code_read",
        reason: None,
        blob: Some(sha.into()),
        text: Some(text),
    }
}
fn blob_sha(bytes: &[u8]) -> String {
    let mut digest = ring::digest::Context::new(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY);
    digest.update(format!("blob {}\0", bytes.len()).as_bytes());
    digest.update(bytes);
    digest
        .finish()
        .as_ref()
        .iter()
        .fold(String::with_capacity(40), |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        })
}

#[cfg(test)]
#[path = "code_tests.rs"]
mod tests;
