use std::collections::HashMap;
use std::sync::Mutex;

use qdrant_client::Payload;
use qdrant_client::qdrant::PointStruct;

use crate::chunking::Chunk;

pub struct QdrantPointStore {
    collection_name: String,
    dimensions: usize,
    points: Mutex<Vec<PointStruct>>,
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

            points.push(PointStruct::new(
                stable_point_id(index as u64, &chunk.id),
                vector,
                Payload::from(payload),
            ));
        }

        Ok(points.len())
    }

    pub fn point_count(&self) -> usize {
        self.points
            .lock()
            .map(|points| points.len())
            .unwrap_or_default()
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
