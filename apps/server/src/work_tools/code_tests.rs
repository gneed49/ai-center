use super::*;
use axum::{Router, extract::State, http::Uri, routing::get};
use std::sync::{Arc, Mutex};
const COMMIT: &str = "1111111111111111111111111111111111111111";
const ROOT: &str = "2222222222222222222222222222222222222222";
const CHILD: &str = "3333333333333333333333333333333333333333";
const TEXT: &str = "// [FICTIF] code\nfn active() -> bool { true }\n";
fn input() -> ReadCode {
    ReadCode {
        connection_id: Uuid::new_v4(),
        repository: "fictif/repo".into(),
        commit_sha: COMMIT.into(),
        paths: vec!["src/lib.rs".into()],
    }
}
fn encoded(bytes: &[u8]) -> Value {
    json!({"sha":blob_sha(bytes),"encoding":"base64","size":bytes.len(),"content":STANDARD.encode(bytes)})
}
#[test]
fn input_and_blob_hashes_bound_what_can_be_called_evidence() {
    let mut valid = input();
    assert!(validate(&mut valid).is_ok());
    for path in [
        "../secrets",
        "/etc/passwd",
        "src//lib.rs",
        "src/./lib.rs",
        "src\\lib.rs",
        "a/b/c/d/e/f/g/h/i",
        ".env.production",
        "config/private.key",
    ] {
        let mut bad = input();
        bad.paths = vec![path.into()];
        assert!(validate(&mut bad).is_err());
    }
    let mut bad = input();
    bad.commit_sha = "main".into();
    assert!(validate(&mut bad).is_err());
    let bytes = TEXT.as_bytes();
    let sha = blob_sha(bytes);
    let result = decode_blob("src/lib.rs", &sha, &encoded(bytes));
    assert_eq!(result.status, "code_read");
    assert_eq!(result.text.as_deref(), Some(TEXT));
    let mut altered = encoded(bytes);
    altered["content"] = json!(STANDARD.encode("different"));
    assert_ne!(
        decode_blob("src/lib.rs", &sha, &altered).status,
        "code_read"
    );
    let bytes = [0_u8, 255];
    assert_eq!(
        decode_blob("binary", &blob_sha(&bytes), &encoded(&bytes)).status,
        "binary"
    );
    let large = vec![b'x'; FILE_LIMIT + 1];
    assert_eq!(
        decode_blob("large", &blob_sha(&large), &encoded(&large)).status,
        "too_large"
    );
}
#[test]
fn sensitive_blob_is_not_retained_as_content_or_evidence() {
    let text = format!("// [FICTIF]\nconst TOKEN = \"ghp_{}\";", "FICTIF".repeat(6));
    let bytes = text.as_bytes();
    let result = decode_blob("src/settings.rs", &blob_sha(bytes), &encoded(bytes));
    assert_eq!(result.status, "unsupported");
    assert_eq!(result.reason, Some("sensitive_content_excluded"));
    assert!(result.text.is_none());
    assert!(result.blob.is_none());
}
#[derive(Clone, Default)]
struct Mock {
    paths: Arc<Mutex<Vec<String>>>,
    unverified: bool,
}
async fn response(State(mock): State<Mock>, uri: Uri) -> axum::Json<Value> {
    let path = uri.path();
    mock.paths.lock().expect("requests").push(path.into());
    let code_sha = blob_sha(TEXT.as_bytes());
    let binary_sha = blob_sha(&[0_u8, 255]);
    let value = if path.ends_with(&format!("/commits/{COMMIT}")) {
        json!({"sha":if mock.unverified{ROOT}else{COMMIT},"tree":{"sha":ROOT}})
    } else if path.ends_with(&format!("/trees/{ROOT}")) {
        json!({"sha":ROOT,"truncated":false,"tree":[
            {"path":"src","type":"tree","mode":"040000","sha":CHILD},
            {"path":"link","type":"blob","mode":"120000","sha":code_sha,"size":TEXT.len()},
            {"path":"large","type":"blob","mode":"100644","sha":code_sha,"size":FILE_LIMIT+1},
            {"path":"binary","type":"blob","mode":"100644","sha":binary_sha,"size":2}]})
    } else if path.ends_with(&format!("/trees/{CHILD}")) {
        json!({"sha":CHILD,"truncated":false,"tree":[{"path":"lib.rs","type":"blob","mode":"100644","sha":code_sha,"size":TEXT.len()}]})
    } else if path.ends_with(&format!("/blobs/{code_sha}")) {
        encoded(TEXT.as_bytes())
    } else if path.ends_with(&format!("/blobs/{binary_sha}")) {
        encoded(&[0_u8, 255])
    } else {
        json!({"unexpected":true})
    };
    axum::Json(value)
}
async fn server(mock: Mock) -> (ToolClient, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let client = ToolClient::loopback(
        &format!("http://{}/", listener.local_addr().expect("addr")),
        Duration::from_secs(1),
    )
    .expect("client");
    let app = Router::new().fallback(get(response)).with_state(mock);
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    (client, task)
}
#[tokio::test]
async fn follows_only_verified_trees_and_never_symlinks_or_mutable_refs() {
    let mock = Mock::default();
    let (client, task) = server(mock.clone()).await;
    let mut input = input();
    input.paths = vec![
        "src/lib.rs".into(),
        "src/missing.rs".into(),
        "link".into(),
        "large".into(),
        "binary".into(),
    ];
    let (verified, files) = collect(&client, &SecretString::from("synthetic-key"), &input).await;
    assert!(verified);
    assert_eq!(
        files.iter().map(|file| file.status).collect::<Vec<_>>(),
        vec!["code_read", "missing", "unsupported", "too_large", "binary"]
    );
    assert_eq!(
        mock.paths.lock().expect("paths").len(),
        5,
        "Trees are shared; rejected targets never fetch content"
    );
    task.abort();
    let mock = Mock {
        unverified: true,
        ..Mock::default()
    };
    let (client, task) = server(mock.clone()).await;
    let (verified, files) = collect(&client, &SecretString::from("synthetic-key"), &input).await;
    assert!(!verified);
    assert!(files.iter().all(|file| file.status == "unavailable"));
    assert_eq!(mock.paths.lock().expect("paths").len(), 1);
    task.abort();
}
