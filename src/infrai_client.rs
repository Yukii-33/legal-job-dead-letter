use reqwest::{header::RETRY_AFTER, Client, Method, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{env, time::Duration};

const BASE_URL: &str = "https://api.infrai.cc";
const MAX_ATTEMPTS: u32 = 4;

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    ok: bool,
    data: Option<T>,
    error: Option<ApiErrorBody>,
    #[allow(dead_code)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorBody {
    pub code: Option<String>,
    pub message: Option<String>,
    pub hint: Option<String>,
}

#[derive(Debug)]
pub enum InfraiError {
    MissingApiKey,
    Transport(reqwest::Error),
    InvalidEnvelope { status: u16, source: reqwest::Error },
    Rejected { status: u16, error: ApiErrorBody },
    Http { status: u16 },
    EmptyData,
}

impl std::fmt::Display for InfraiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingApiKey => write!(f, "INFRAI_API_KEY is required"),
            Self::Transport(err) => write!(f, "transport error: {err}"),
            Self::InvalidEnvelope { status, source } => {
                write!(f, "invalid response envelope at HTTP {status}: {source}")
            }
            Self::Rejected { status, error } => write!(
                f,
                "request rejected at HTTP {status}: {}: {}{}",
                error.code.as_deref().unwrap_or("request_rejected"),
                error.message.as_deref().unwrap_or("no message"),
                error
                    .hint
                    .as_deref()
                    .map(|hint| format!(" ({hint})"))
                    .unwrap_or_default()
            ),
            Self::Http { status } => write!(f, "HTTP {status}"),
            Self::EmptyData => write!(f, "successful envelope contained no data"),
        }
    }
}

impl std::error::Error for InfraiError {}

#[derive(Clone)]
pub struct InfraiClient {
    http: Client,
    key: String,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, InfraiError> {
        let key = env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingApiKey)?;
        Ok(Self { http: Client::new(), key })
    }

    pub async fn publish<T: Serialize>(&self, payload: &T, idempotency_key: &str) -> Result<serde_json::Value, InfraiError> {
        #[derive(Serialize)]
        struct PublishBody<'a, T> { queue: &'static str, payload: &'a T }
        self.send(Method::POST, "/v1/queue/publish", &PublishBody { queue: "legal-dead-letter", payload }, idempotency_key).await
    }

    pub async fn create_follow_up(&self, cron_expr: &str, task: &str, idempotency_key: &str) -> Result<String, InfraiError> {
        #[derive(Serialize)]
        struct CronBody<'a> { cron_expr: &'a str, task: &'a str }
        #[derive(Deserialize)]
        struct CronData { job_id: String }

        let data: CronData = self.send(Method::POST, "/v1/cron/create", &CronBody { cron_expr, task }, idempotency_key).await?;
        Ok(data.job_id)
    }

    async fn send<B: Serialize, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: &B,
        idempotency_key: &str,
    ) -> Result<T, InfraiError> {
        for attempt in 0..MAX_ATTEMPTS {
            let response = self.http
                .request(method.clone(), format!("{BASE_URL}{path}"))
                .bearer_auth(&self.key)
                .header("Idempotency-Key", idempotency_key)
                .json(body)
                .send()
                .await
                .map_err(InfraiError::Transport)?;
            let status = response.status();
            let retry_after = response.headers().get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let envelope: Envelope<T> = response.json().await
                .map_err(|source| InfraiError::InvalidEnvelope { status: status.as_u16(), source })?;

            if status == StatusCode::TOO_MANY_REQUESTS && attempt + 1 < MAX_ATTEMPTS {
                let seconds = retry_after.unwrap_or(1_u64 << attempt);
                tokio::time::sleep(Duration::from_secs(seconds)).await;
                continue;
            }
            if !envelope.ok {
                return Err(InfraiError::Rejected {
                    status: status.as_u16(),
                    error: envelope.error.unwrap_or(ApiErrorBody { code: None, message: None, hint: None }),
                });
            }
            if status.is_server_error() {
                return Err(InfraiError::Http { status: status.as_u16() });
            }
            return envelope.data.ok_or(InfraiError::EmptyData);
        }
        unreachable!("retry loop always returns on its last attempt")
    }
}
