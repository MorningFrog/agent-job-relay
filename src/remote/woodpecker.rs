//! Read-only Woodpecker 3.x API observation. A restart creates a different pipeline.
use anyhow::{Context, Result, ensure};
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;
use tokio::io::AsyncReadExt;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WoodpeckerSpec {
    /// Explicit server origin, including a port when needed. HTTP is loopback-only.
    pub server_url: String,
    pub repository: String,
    pub repo_id: u64,
    /// Repository-local number (the number in the Web URL).
    pub pipeline_number: u64,
    /// Global API pipeline ID, distinct from its repository-local number.
    pub pipeline_id: u64,
    /// Full source commit from the selected pipeline, never a branch selector.
    pub commit: String,
    /// Existing private file containing a bearer token, not the token itself.
    /// When omitted, use WOODPECKER_TOKEN if present, otherwise an anonymous GET.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_file: Option<PathBuf>,
}

impl WoodpeckerSpec {
    pub fn validate(&self) -> Result<()> {
        let url = reqwest::Url::parse(&self.server_url)
            .map_err(|_| anyhow::anyhow!("invalid Woodpecker server origin"))?;
        ensure!(
            self.server_url.len() <= 2048
                && !self.server_url.chars().any(char::is_whitespace)
                && url.has_host()
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none()
                && url.path() == "/"
                && self.origin() == url.origin().ascii_serialization()
                && (url.scheme() == "https"
                    || (url.scheme() == "http"
                        && url.host_str().is_some_and(|host| {
                            host == "localhost"
                                || host
                                    .trim_matches(['[', ']'])
                                    .parse::<std::net::IpAddr>()
                                    .is_ok_and(|ip| ip.is_loopback())
                        }))),
            "Woodpecker server must be an HTTPS origin or loopback HTTP origin without credentials, path, query or fragment"
        );
        let parts: Vec<_> = self.repository.split('/').collect();
        ensure!(
            self.repository.len() <= 200
                && (2..=10).contains(&parts.len())
                && parts.iter().all(|s| !s.is_empty()
                    && *s != "."
                    && *s != ".."
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))),
            "invalid Woodpecker repository full name"
        );
        ensure!(
            [self.repo_id, self.pipeline_number, self.pipeline_id]
                .iter()
                .all(|id| *id > 0 && *id <= i64::MAX as u64)
                && [40, 64].contains(&self.commit.len())
                && self.commit.bytes().all(|b| b.is_ascii_hexdigit()),
            "Woodpecker observation requires positive repository/pipeline IDs, pipeline number and a full commit"
        );
        if let Some(path) = &self.token_file {
            let text = path
                .to_str()
                .context("invalid Woodpecker token file path")?;
            ensure!(
                path.is_absolute() && text.len() <= 4096 && !text.contains(['\0', '\n', '\r']),
                "Woodpecker token file must be an absolute path"
            );
        }
        Ok(())
    }

    fn origin(&self) -> &str {
        self.server_url.trim_end_matches('/')
    }

    fn url(&self) -> String {
        format!(
            "{}/repos/{}/pipeline/{}",
            self.origin(),
            self.repo_id,
            self.pipeline_number
        )
    }
}

async fn token(spec: &WoodpeckerSpec) -> Result<Option<reqwest::header::HeaderValue>> {
    let value = if let Some(path) = &spec.token_file {
        ensure!(
            tokio::fs::metadata(path)
                .await
                .is_ok_and(|metadata| metadata.is_file()),
            "Woodpecker credential file unavailable or not a regular file"
        );
        let file = tokio::fs::File::open(path)
            .await
            .map_err(|_| anyhow::anyhow!("Woodpecker credential file unavailable"))?;
        let mut bytes = Vec::new();
        file.take(8193)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| anyhow::anyhow!("Woodpecker credential file unreadable"))?;
        ensure!(bytes.len() <= 8192, "Woodpecker credential exceeds limit");
        Some(
            String::from_utf8(bytes)
                .map_err(|_| anyhow::anyhow!("invalid Woodpecker credential"))?,
        )
    } else {
        match std::env::var("WOODPECKER_TOKEN") {
            Ok(value) => Some(value),
            Err(std::env::VarError::NotPresent) => None,
            Err(_) => anyhow::bail!("invalid Woodpecker credential"),
        }
    };
    value
        .map(|value| {
            let value = value.trim();
            ensure!(
                !value.is_empty()
                    && value.len() <= 8192
                    && value.bytes().all(|b| b.is_ascii_graphic()),
                "invalid Woodpecker credential"
            );
            let mut header = reqwest::header::HeaderValue::from_str(&format!("Bearer {value}"))
                .map_err(|_| anyhow::anyhow!("invalid Woodpecker credential"))?;
            header.set_sensitive(true);
            Ok(header)
        })
        .transpose()
}

async fn get(
    client: &reqwest::Client,
    url: String,
    token: &Option<reqwest::header::HeaderValue>,
) -> Result<Value> {
    let mut request = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json");
    if let Some(token) = token {
        request = request.header(reqwest::header::AUTHORIZATION, token.clone());
    }
    // No response body, URL, token or underlying HTTP diagnostic enters errors.
    let mut response = request.send().await.map_err(|_| {
        anyhow::anyhow!("Woodpecker observation unavailable: transport_unavailable")
    })?;
    let status = response.status();
    ensure!(
        status.is_success(),
        "Woodpecker observation unavailable: {}",
        match status.as_u16() {
            401 | 403 => "authentication_or_permission",
            404 => "not_found_or_not_authorized",
            429 => "rate_limited",
            300..=399 => "redirect_refused",
            _ => "server_unavailable",
        }
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| anyhow::anyhow!("Woodpecker observation unavailable: response_unavailable"))?
    {
        ensure!(
            bytes.len().saturating_add(chunk.len()) <= super::transport::MAX_RESPONSE,
            "Woodpecker response exceeds limit"
        );
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!("invalid Woodpecker API response"))
}

pub(crate) async fn sample(spec: &WoodpeckerSpec) -> Result<Value> {
    spec.validate()?;
    let token = token(spec).await?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(super::transport::DEADLINE)
        .build()
        .map_err(|_| anyhow::anyhow!("Woodpecker HTTP client unavailable"))?;
    let repo = get(
        &client,
        format!("{}/api/repos/{}", spec.origin(), spec.repo_id),
        &token,
    )
    .await?;
    ensure!(
        repo["id"].as_u64() == Some(spec.repo_id) && repo["full_name"] == spec.repository,
        "Woodpecker repository identity mismatch"
    );
    let value = get(
        &client,
        format!(
            "{}/api/repos/{}/pipelines/{}",
            spec.origin(),
            spec.repo_id,
            spec.pipeline_number
        ),
        &token,
    )
    .await?;
    pipeline(spec, &value)
}

fn pipeline(spec: &WoodpeckerSpec, value: &Value) -> Result<Value> {
    ensure!(
        value["id"].as_u64() == Some(spec.pipeline_id)
            && value["number"].as_u64() == Some(spec.pipeline_number)
            && value["commit"]
                .as_str()
                .is_some_and(|commit| commit.eq_ignore_ascii_case(&spec.commit)),
        "Woodpecker pipeline identity mismatch"
    );
    let status = value["status"]
        .as_str()
        .context("missing Woodpecker pipeline status")?;
    ensure!(
        valid_status(status),
        "unsupported Woodpecker pipeline status"
    );
    let mut result = json!({
        "server_url":spec.origin(), "repository":spec.repository, "repo_id":spec.repo_id,
        "pipeline_id":spec.pipeline_id, "pipeline_number":spec.pipeline_number, "commit":spec.commit,
        "status":status, "url":spec.url(), "sampled_at":crate::core::now(),
        "can_cancel":false, "mode":"observe_only"
    });
    for key in ["created", "started", "finished", "parent"] {
        if let Some(n) = value[key].as_u64() {
            result[key] = json!(n);
        }
    }
    // Detail is optional and cannot invalidate authoritative exact-pipeline status.
    match workflows(value) {
        Ok(workflows) => result["workflows"] = workflows,
        Err(error) => result["workflows_error"] = json!(error.to_string()),
    }
    Ok(result)
}

fn valid_status(status: &str) -> bool {
    [
        "blocked", "declined", "pending", "waiting", "running", "success", "failure", "error",
        "killed", "skipped",
    ]
    .contains(&status)
}

fn workflows(value: &Value) -> Result<Value> {
    let list = value["workflows"]
        .as_array()
        .context("missing Woodpecker workflows")?;
    let mut items = Vec::new();
    let bounded = |value: &Value| {
        value
            .as_str()
            .unwrap_or("")
            .chars()
            .take(160)
            .collect::<String>()
    };
    let mut identities = std::collections::HashSet::new();
    for workflow in list.iter().take(20) {
        let id = workflow["id"]
            .as_u64()
            .filter(|id| *id > 0)
            .context("missing Woodpecker workflow identity")?;
        ensure!(
            identities.insert(id),
            "duplicate Woodpecker workflow identity"
        );
        let state = workflow["state"]
            .as_str()
            .context("missing Woodpecker workflow state")?;
        ensure!(valid_status(state), "unsupported Woodpecker workflow state");
        // Pending/filtered workflows may not yet have a children array.
        let steps = workflow
            .get("children")
            .filter(|v| !v.is_null())
            .map(|v| v.as_array().context("invalid Woodpecker steps"))
            .transpose()?;
        let current = steps.and_then(|steps| steps.iter().find(|s| s["state"] == "running"));
        items.push(json!({"id":id,"name":bounded(&workflow["name"]),"status":state,
            "completed_steps":steps.map(|steps| steps.iter().filter(|s| matches!(s["state"].as_str(),Some("success"|"failure"|"error"|"killed"|"skipped"|"declined"))).count()),
            "total_steps":steps.map(Vec::len), "current_step":current.map(|s| bounded(&s["name"]))}));
    }
    Ok(json!({"items":items,"total_count":list.len(),"truncated":list.len()>20}))
}

pub(crate) fn terminal_state(value: &Value) -> Option<&'static str> {
    match value["status"].as_str()? {
        "success" | "skipped" => Some("completed"),
        "failure" | "error" | "declined" => Some("failed"),
        "killed" => Some("cancelled"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn http_reads_bound_responses_refuse_redirects_and_redact_error_bodies() {
        use tokio::io::AsyncWriteExt;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap();
        for (status, body, expected) in [
            (
                403,
                "private-test-secret".to_string(),
                "authentication_or_permission",
            ),
            (
                404,
                "private-test-secret".into(),
                "not_found_or_not_authorized",
            ),
            (429, "private-test-secret".into(), "rate_limited"),
            (302, "private-test-secret".into(), "redirect_refused"),
            (
                200,
                "private-test-secret".into(),
                "invalid Woodpecker API response",
            ),
            (
                200,
                "x".repeat(super::super::transport::MAX_RESPONSE + 1),
                "response exceeds limit",
            ),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}/api/repos/1", listener.local_addr().unwrap());
            let worker = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                loop {
                    let byte = stream.read_u8().await.unwrap();
                    bytes.push(byte);
                    if bytes.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8(bytes).unwrap();
                assert!(request.starts_with("GET /api/repos/1 HTTP/1.1"));
                assert!(request.contains("authorization: Bearer private-test-secret"));
                let response = format!(
                    "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nLocation: /must-not-follow\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
                // A redirected request would fail or remain visible here.
                assert!(
                    tokio::time::timeout(std::time::Duration::from_millis(50), listener.accept())
                        .await
                        .is_err()
                );
            });
            let credential = Some(reqwest::header::HeaderValue::from_static(
                "Bearer private-test-secret",
            ));
            let error = get(&client, url, &credential)
                .await
                .unwrap_err()
                .to_string();
            assert!(error.contains(expected), "{error}");
            assert!(!error.contains("private-test-secret"));
            worker.await.unwrap();
        }
    }

    fn spec() -> WoodpeckerSpec {
        WoodpeckerSpec {
            server_url: "https://ci.example.test".into(),
            repository: "owner/repo".into(),
            repo_id: 1,
            pipeline_number: 42,
            pipeline_id: 69,
            commit: "a".repeat(40),
            token_file: None,
        }
    }

    #[test]
    fn pipeline_identity_status_and_bounded_optional_details() {
        let spec = spec();
        let mut value = json!({"id":69,"number":42,"commit":spec.commit,"status":"running",
            "workflows":[{"id":9,"name":"步".repeat(200),"state":"running", "environ":{"SECRET":"must-not-retain"},
                "children":[{"id":1,"state":"success","name":"clone"},{"id":2,"state":"running","name":"编".repeat(200)},{"id":3,"state":"pending"}]}]});
        let result = pipeline(&spec, &value).unwrap();
        assert_eq!(result["workflows"]["items"][0]["completed_steps"], 1);
        assert_eq!(result["workflows"]["items"][0]["total_steps"], 3);
        for key in ["name", "current_step"] {
            assert_eq!(
                result["workflows"]["items"][0][key]
                    .as_str()
                    .unwrap()
                    .chars()
                    .count(),
                160
            );
        }
        assert!(!result.to_string().contains("must-not-retain"));
        for (key, wrong) in [
            ("id", json!(70)),
            ("number", json!(43)),
            ("commit", json!("b".repeat(40))),
        ] {
            let mut wrong_value = value.clone();
            wrong_value[key] = wrong;
            assert!(
                pipeline(&spec, &wrong_value)
                    .unwrap_err()
                    .to_string()
                    .contains("identity mismatch")
            );
        }
        for (status, expected) in [
            ("pending", None),
            ("waiting", None),
            ("running", None),
            ("blocked", None),
            ("success", Some("completed")),
            ("skipped", Some("completed")),
            ("failure", Some("failed")),
            ("error", Some("failed")),
            ("declined", Some("failed")),
            ("killed", Some("cancelled")),
        ] {
            value["status"] = json!(status);
            assert_eq!(terminal_state(&pipeline(&spec, &value).unwrap()), expected);
        }
        value["status"] = json!("unknown");
        assert!(pipeline(&spec, &value).is_err());
        value["status"] = json!("success");
        value["workflows"] = json!({});
        let result = pipeline(&spec, &value).unwrap();
        assert_eq!(terminal_state(&result), Some("completed"));
        assert_eq!(result["workflows_error"], "missing Woodpecker workflows");
        value["workflows"] = json!(
            (1..=25)
                .map(|id| json!({"id":id,"name":"test","state":"pending"}))
                .collect::<Vec<_>>()
        );
        let result = pipeline(&spec, &value).unwrap();
        assert_eq!(result["workflows"]["items"].as_array().unwrap().len(), 20);
        assert_eq!(result["workflows"]["total_count"], 25);
        assert_eq!(result["workflows"]["truncated"], true);
        assert_eq!(result["workflows"]["items"][0]["total_steps"], Value::Null);
    }

    #[test]
    fn targets_reject_credentials_moving_selectors_and_insecure_nonlocal_servers() {
        let mut spec = spec();
        for url in [
            "https://user:secret@ci.example.test",
            "https://ci.example.test/?token=x",
            "https://ci.example.test/#x",
            "https://ci.example.test/path",
            "https://ci.example.test/path/..",
            "http://ci.example.test",
            "ftp://localhost",
            "https://ci.example.test/\n",
        ] {
            spec.server_url = url.into();
            assert!(spec.validate().is_err(), "{url}");
        }
        for url in [
            "https://ci.example.test",
            "https://ci.example.test/",
            "http://127.0.0.1:8000",
            "http://[::1]:8000",
            "http://localhost",
        ] {
            spec.server_url = url.into();
            spec.validate().unwrap();
        }
        spec.pipeline_number = 0;
        assert!(spec.validate().is_err());
        spec.pipeline_number = 42;
        spec.commit = "main".into();
        assert!(spec.validate().is_err());
    }
}
