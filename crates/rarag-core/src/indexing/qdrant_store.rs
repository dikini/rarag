use std::collections::HashMap;
use std::sync::Mutex;

use qdrant_client::qdrant::{
    Condition, CountPointsBuilder, CreateCollectionBuilder, DeletePointsBuilder, Distance,
    PointStruct, SearchPointsBuilder, VectorParamsBuilder,
};
use qdrant_client::{Payload, Qdrant};

use crate::chunking::Chunk;

#[derive(Debug, Clone, PartialEq)]
pub struct VectorSearchHit {
    pub snapshot_id: String,
    pub chunk_id: String,
    pub symbol_path: Option<String>,
    pub score: f32,
}

#[derive(Debug, Clone)]
struct PreparedPoint {
    point: PointStruct,
    snapshot_id: String,
    chunk_id: String,
    symbol_path: Option<String>,
    vector: Vec<f32>,
}

enum QdrantBackend {
    Remote { client: Qdrant },
    InMemory { points: Mutex<Vec<PreparedPoint>> },
}

pub struct QdrantPointStore {
    endpoint: String,
    collection_name: String,
    dimensions: usize,
    backend: QdrantBackend,
}

impl QdrantPointStore {
    pub fn new(
        endpoint: impl Into<String>,
        collection_name: impl Into<String>,
        dimensions: usize,
    ) -> Result<Self, String> {
        let endpoint = endpoint.into();
        if dimensions == 0 {
            return Err("dimensions must be greater than zero".to_string());
        }
        if endpoint.trim().is_empty() {
            return Err("qdrant endpoint must not be empty".to_string());
        }
        if endpoint.starts_with("memory://") {
            return Ok(Self::new_in_memory(
                endpoint,
                collection_name.into(),
                dimensions,
            ));
        }

        let client = Qdrant::from_url(&endpoint)
            .build()
            .map_err(|err| err.to_string())?;
        Ok(Self {
            endpoint,
            collection_name: collection_name.into(),
            dimensions,
            backend: QdrantBackend::Remote { client },
        })
    }

    pub fn new_in_memory(
        endpoint: impl Into<String>,
        collection_name: impl Into<String>,
        dimensions: usize,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            collection_name: collection_name.into(),
            dimensions,
            backend: QdrantBackend::InMemory {
                points: Mutex::new(Vec::new()),
            },
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub async fn replace_snapshot(
        &self,
        snapshot_id: &str,
        chunks: &[Chunk],
        vectors: Vec<Vec<f32>>,
    ) -> Result<usize, String> {
        validate_vectors(&self.collection_name, self.dimensions, chunks, &vectors)?;

        match &self.backend {
            QdrantBackend::Remote { client } => {
                self.ensure_remote_collection(client).await?;
                client
                    .delete_points(
                        DeletePointsBuilder::new(self.collection_name.clone())
                            .points(qdrant_client::qdrant::Filter::must([Condition::matches(
                                "snapshot_id",
                                snapshot_id.to_string(),
                            )]))
                            .wait(true),
                    )
                    .await
                    .map_err(|err| err.to_string())?;

                let points = build_points(snapshot_id, chunks, vectors);
                if points.is_empty() {
                    return Ok(0);
                }
                client
                    .upsert_points(
                        qdrant_client::qdrant::UpsertPointsBuilder::new(
                            self.collection_name.clone(),
                            points,
                        )
                        .wait(true),
                    )
                    .await
                    .map_err(|err| err.to_string())?;
                Ok(chunks.len())
            }
            QdrantBackend::InMemory { points } => {
                let mut stored = points
                    .lock()
                    .map_err(|_| "qdrant points lock poisoned".to_string())?;
                stored.retain(|point| point.snapshot_id != snapshot_id);
                let added = chunks.len();
                stored.extend(chunks.iter().zip(vectors).enumerate().map(
                    |(index, (chunk, vector))| PreparedPoint {
                        point: build_point(snapshot_id, index as u64, chunk, vector.clone()),
                        snapshot_id: snapshot_id.to_string(),
                        chunk_id: chunk.id.clone(),
                        symbol_path: chunk.symbol_path.clone(),
                        vector,
                    },
                ));
                Ok(added)
            }
        }
    }

    pub async fn search_snapshot(
        &self,
        snapshot_id: &str,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<VectorSearchHit>, String> {
        if query_vector.len() != self.dimensions {
            return Err(format!(
                "query vector dimensions did not match collection {} dimensions {}",
                self.collection_name, self.dimensions
            ));
        }

        match &self.backend {
            QdrantBackend::Remote { client } => {
                self.ensure_remote_collection(client).await?;
                let response = client
                    .search_points(
                        SearchPointsBuilder::new(
                            self.collection_name.clone(),
                            query_vector.to_vec(),
                            limit as u64,
                        )
                        .filter(qdrant_client::qdrant::Filter::must([Condition::matches(
                            "snapshot_id",
                            snapshot_id.to_string(),
                        )]))
                        .with_payload(true),
                    )
                    .await
                    .map_err(|err| err.to_string())?;

                Ok(response
                    .result
                    .into_iter()
                    .filter_map(|point| {
                        let chunk_id = payload_string(&point.payload, "chunk_id")?;
                        Some(VectorSearchHit {
                            snapshot_id: payload_string(&point.payload, "snapshot_id")
                                .unwrap_or_else(|| snapshot_id.to_string()),
                            chunk_id,
                            symbol_path: payload_string(&point.payload, "symbol_path"),
                            score: point.score,
                        })
                    })
                    .collect())
            }
            QdrantBackend::InMemory { points } => {
                let points = points
                    .lock()
                    .map_err(|_| "qdrant points lock poisoned".to_string())?;
                let mut hits: Vec<_> = points
                    .iter()
                    .filter(|point| point.snapshot_id == snapshot_id)
                    .map(|point| VectorSearchHit {
                        snapshot_id: point.snapshot_id.clone(),
                        chunk_id: point.chunk_id.clone(),
                        symbol_path: point.symbol_path.clone(),
                        score: cosine_similarity(&point.vector, query_vector),
                    })
                    .collect();
                hits.sort_by(|left, right| right.score.total_cmp(&left.score));
                hits.truncate(limit);
                Ok(hits)
            }
        }
    }

    pub async fn point_count(&self) -> Result<usize, String> {
        match &self.backend {
            QdrantBackend::Remote { client } => {
                self.ensure_remote_collection(client).await?;
                let response = client
                    .count(CountPointsBuilder::new(self.collection_name.clone()).exact(true))
                    .await
                    .map_err(|err| err.to_string())?;
                usize::try_from(response.result.unwrap_or_default().count)
                    .map_err(|err| err.to_string())
            }
            QdrantBackend::InMemory { points } => points
                .lock()
                .map(|points| points.len())
                .map_err(|_| "qdrant points lock poisoned".to_string()),
        }
    }

    pub async fn prepared_point_count(&self) -> Result<usize, String> {
        match &self.backend {
            QdrantBackend::Remote { .. } => self.point_count().await,
            QdrantBackend::InMemory { points } => points
                .lock()
                .map(|points| {
                    points
                        .iter()
                        .filter(|point| point.point.id.is_some())
                        .count()
                })
                .map_err(|_| "qdrant points lock poisoned".to_string()),
        }
    }

    async fn ensure_remote_collection(&self, client: &Qdrant) -> Result<(), String> {
        let exists = client
            .collection_exists(&self.collection_name)
            .await
            .map_err(|err| err.to_string())?;
        if exists {
            return Ok(());
        }

        client
            .create_collection(
                CreateCollectionBuilder::new(self.collection_name.clone()).vectors_config(
                    VectorParamsBuilder::new(self.dimensions as u64, Distance::Cosine),
                ),
            )
            .await
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

fn validate_vectors(
    collection_name: &str,
    dimensions: usize,
    chunks: &[Chunk],
    vectors: &[Vec<f32>],
) -> Result<(), String> {
    if vectors.iter().any(|vector| vector.len() != dimensions) {
        return Err(format!(
            "vector dimensions did not match collection {} dimensions {}",
            collection_name, dimensions
        ));
    }
    if vectors.len() != chunks.len() {
        return Err("vector count did not match chunk count".to_string());
    }
    Ok(())
}

fn build_points(snapshot_id: &str, chunks: &[Chunk], vectors: Vec<Vec<f32>>) -> Vec<PointStruct> {
    chunks
        .iter()
        .zip(vectors)
        .enumerate()
        .map(|(index, (chunk, vector))| build_point(snapshot_id, index as u64, chunk, vector))
        .collect()
}

fn build_point(snapshot_id: &str, seed: u64, chunk: &Chunk, vector: Vec<f32>) -> PointStruct {
    let mut payload = HashMap::new();
    payload.insert(
        "chunk_id".to_string(),
        serde_json::Value::String(chunk.id.clone()),
    );
    payload.insert(
        "snapshot_id".to_string(),
        serde_json::Value::String(snapshot_id.to_string()),
    );
    if let Some(symbol_path) = &chunk.symbol_path {
        payload.insert(
            "symbol_path".to_string(),
            serde_json::Value::String(symbol_path.clone()),
        );
    }

    PointStruct::new(
        stable_point_id(seed, &chunk.id),
        vector,
        Payload::from(payload),
    )
}

fn payload_string(
    payload: &HashMap<String, qdrant_client::qdrant::Value>,
    key: &str,
) -> Option<String> {
    let value = payload.get(key)?;
    let value = serde_json::to_value(value).ok()?;
    value
        .get("kind")?
        .get("stringValue")?
        .as_str()
        .map(str::to_string)
}

fn stable_point_id(seed: u64, chunk_id: &str) -> u64 {
    let mut hash = 1469598103934665603u64 ^ seed;
    for byte in chunk_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    let numerator: f32 = left.iter().zip(right).map(|(a, b)| a * b).sum();
    let left_norm: f32 = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm: f32 = right.iter().map(|value| value * value).sum::<f32>().sqrt();

    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        numerator / (left_norm * right_norm)
    }
}
