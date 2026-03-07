use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

pub trait EmbeddingProvider {
    fn embed_texts(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, String>;
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleEmbeddings {
    base_url: String,
    model: String,
    api_key_env: String,
    dimensions: usize,
    client: reqwest::Client,
}

impl OpenAiCompatibleEmbeddings {
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key_env: impl Into<String>,
        dimensions: usize,
    ) -> Result<Self, String> {
        let provider = Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            api_key_env: api_key_env.into(),
            dimensions,
            client: reqwest::Client::new(),
        };

        if provider.base_url.is_empty() {
            return Err("base_url must not be empty".to_string());
        }
        if provider.model.trim().is_empty() {
            return Err("model must not be empty".to_string());
        }
        if provider.api_key_env.trim().is_empty() {
            return Err("api_key_env must not be empty".to_string());
        }
        if provider.dimensions == 0 {
            return Err("dimensions must be greater than zero".to_string());
        }

        Ok(provider)
    }

    pub fn build_request(&self, inputs: &[String]) -> Result<reqwest::Request, String> {
        let token = std::env::var(&self.api_key_env)
            .map_err(|_| format!("missing environment variable {}", self.api_key_env))?;

        self.client
            .post(format!("{}/embeddings", self.base_url))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(CONTENT_TYPE, "application/json")
            .json(&OpenAiCompatibleEmbeddingRequest {
                model: self.model.clone(),
                input: inputs.to_vec(),
                dimensions: self.dimensions,
            })
            .build()
            .map_err(|err| err.to_string())
    }
}

impl EmbeddingProvider for OpenAiCompatibleEmbeddings {
    fn embed_texts(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let request = self.build_request(inputs)?;
        run_embedding_request(self.client.clone(), request)
    }
}

#[derive(Debug, Clone, Serialize)]
struct OpenAiCompatibleEmbeddingRequest {
    model: String,
    input: Vec<String>,
    dimensions: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiCompatibleEmbeddingResponse {
    data: Vec<EmbeddingItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct EmbeddingItem {
    embedding: Vec<f32>,
}

fn run_embedding_request(
    client: reqwest::Client,
    request: reqwest::Request,
) -> Result<Vec<Vec<f32>>, String> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(fetch_embeddings(client, request)))
    } else {
        let runtime = tokio::runtime::Runtime::new().map_err(|err| err.to_string())?;
        runtime.block_on(fetch_embeddings(client, request))
    }
}

async fn fetch_embeddings(
    client: reqwest::Client,
    request: reqwest::Request,
) -> Result<Vec<Vec<f32>>, String> {
    let response = client
        .execute(request)
        .await
        .map_err(|err| err.to_string())?;
    let response = response.error_for_status().map_err(|err| err.to_string())?;
    let payload: OpenAiCompatibleEmbeddingResponse =
        response.json().await.map_err(|err| err.to_string())?;
    Ok(payload
        .data
        .into_iter()
        .map(|item| item.embedding)
        .collect())
}
