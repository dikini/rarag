use std::collections::HashMap;
use std::sync::Mutex;

use qdrant_client::Payload;
use qdrant_client::qdrant::PointStruct;

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

pub struct QdrantPointStore {
    collection_name: String,
    dimensions: usize,
    points: Mutex<Vec<PreparedPoint>>,
}

impl QdrantPointStore {
    pub fn new(collection_name: impl Into<String>, dimensions: usize) -> Self {
        Self {
            collection_name: collection_name.into(),
            dimensions,
            points: Mutex::new(Vec::new()),
        }
    }

    pub fn prepare_points(
        &self,
        _snapshot_id: &str,
        chunks: &[Chunk],
        vectors: Vec<Vec<f32>>,
    ) -> Result<usize, String> {
        if vectors.iter().any(|vector| vector.len() != self.dimensions) {
            return Err(format!(
                "vector dimensions did not match collection {} dimensions {}",
                self.collection_name, self.dimensions
            ));
        }
        if vectors.len() != chunks.len() {
            return Err("vector count did not match chunk count".to_string());
        }

        let mut points = self
            .points
            .lock()
            .map_err(|_| "qdrant points lock poisoned".to_string())?;
        points.clear();
        for (index, (chunk, vector)) in chunks.iter().zip(vectors).enumerate() {
            let mut payload = HashMap::new();
            payload.insert(
                "chunk_id".to_string(),
                serde_json::Value::String(chunk.id.clone()),
            );
            payload.insert(
                "snapshot_id".to_string(),
                serde_json::Value::String(_snapshot_id.to_string()),
            );
            if let Some(symbol_path) = &chunk.symbol_path {
                payload.insert(
                    "symbol_path".to_string(),
                    serde_json::Value::String(symbol_path.clone()),
                );
            }

            points.push(PreparedPoint {
                point: PointStruct::new(
                    stable_point_id(index as u64, &chunk.id),
                    vector.clone(),
                    Payload::from(payload),
                ),
                snapshot_id: _snapshot_id.to_string(),
                chunk_id: chunk.id.clone(),
                symbol_path: chunk.symbol_path.clone(),
                vector,
            });
        }

        Ok(points.len())
    }

    pub fn search_snapshot(
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

        let points = self
            .points
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

    pub fn point_count(&self) -> usize {
        self.points
            .lock()
            .map(|points| points.len())
            .unwrap_or_default()
    }

    pub fn prepared_point_count(&self) -> Result<usize, String> {
        self.points
            .lock()
            .map(|points| {
                points
                    .iter()
                    .filter(|point| point.point.id.is_some())
                    .count()
            })
            .map_err(|_| "qdrant points lock poisoned".to_string())
    }
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
