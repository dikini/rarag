use std::sync::Arc;

use arrow_array::types::Float32Type;
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, Float64Array, RecordBatch, RecordBatchIterator,
    StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::index::vector::IvfFlatIndexBuilder;
use lancedb::index::Index;
use lancedb::query::{ExecutableQuery, QueryBase};

use crate::chunking::Chunk;
use crate::config::VectorDistanceMetric;

#[derive(Debug, Clone, PartialEq)]
pub struct VectorSearchHit {
    pub snapshot_id: String,
    pub chunk_id: String,
    pub symbol_path: Option<String>,
    pub score: f32,
}

#[derive(Debug, Clone)]
struct PreparedPoint {
    snapshot_id: String,
    chunk_id: String,
    symbol_path: Option<String>,
    vector: Vec<f32>,
}

enum LanceDbBackend {
    Local,
    InMemory { points: std::sync::Mutex<Vec<PreparedPoint>> },
}

pub struct LanceDbPointStore {
    db_root: String,
    table_name: String,
    dimensions: usize,
    distance_metric: VectorDistanceMetric,
    backend: LanceDbBackend,
}

impl LanceDbPointStore {
    pub fn new_with_metric(
        db_root: impl Into<String>,
        table_name: impl Into<String>,
        dimensions: usize,
        distance_metric: VectorDistanceMetric,
    ) -> Result<Self, String> {
        let db_root = db_root.into();
        if dimensions == 0 {
            return Err("dimensions must be greater than zero".to_string());
        }
        if db_root.trim().is_empty() {
            return Err("lancedb db_root must not be empty".to_string());
        }

        Ok(Self {
            db_root,
            table_name: table_name.into(),
            dimensions,
            distance_metric,
            backend: LanceDbBackend::Local,
        })
    }

    pub fn new(
        db_root: impl Into<String>,
        table_name: impl Into<String>,
        dimensions: usize,
    ) -> Result<Self, String> {
        Self::new_with_metric(db_root, table_name, dimensions, VectorDistanceMetric::Cosine)
    }

    pub fn new_in_memory_with_metric(
        db_root: impl Into<String>,
        table_name: impl Into<String>,
        dimensions: usize,
        distance_metric: VectorDistanceMetric,
    ) -> Self {
        Self {
            db_root: db_root.into(),
            table_name: table_name.into(),
            dimensions,
            distance_metric,
            backend: LanceDbBackend::InMemory {
                points: std::sync::Mutex::new(Vec::new()),
            },
        }
    }

    pub fn new_in_memory(
        db_root: impl Into<String>,
        table_name: impl Into<String>,
        dimensions: usize,
    ) -> Self {
        Self::new_in_memory_with_metric(
            db_root,
            table_name,
            dimensions,
            VectorDistanceMetric::Cosine,
        )
    }

    pub fn db_root(&self) -> &str {
        &self.db_root
    }

    pub async fn replace_snapshot(
        &self,
        snapshot_id: &str,
        chunks: &[Chunk],
        vectors: Vec<Vec<f32>>,
    ) -> Result<usize, String> {
        validate_vectors(&self.table_name, self.dimensions, chunks, &vectors)?;

        match &self.backend {
            LanceDbBackend::InMemory { points } => {
                let mut stored = points
                    .lock()
                    .map_err(|_| "lancedb points lock poisoned".to_string())?;
                stored.retain(|point| point.snapshot_id != snapshot_id);
                let added = chunks.len();
                stored.extend(chunks.iter().zip(vectors).map(|(chunk, vector)| PreparedPoint {
                    snapshot_id: snapshot_id.to_string(),
                    chunk_id: chunk.id.clone(),
                    symbol_path: chunk.symbol_path.clone(),
                    vector,
                }));
                Ok(added)
            }
            LanceDbBackend::Local => {
                let table = self.ensure_table().await?;
                table
                    .delete(&snapshot_filter(snapshot_id))
                    .await
                    .map_err(|err| err.to_string())?;

                if chunks.is_empty() {
                    return Ok(0);
                }

                let batch = self.build_batch(snapshot_id, chunks, vectors)?;
                let schema = batch.schema();
                let data = Box::new(RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema));
                table
                    .add(data)
                    .execute()
                    .await
                    .map_err(|err| err.to_string())?;
                self.ensure_vector_index(&table).await?;

                Ok(chunks.len())
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
                "query vector dimensions did not match table {} dimensions {}",
                self.table_name, self.dimensions
            ));
        }

        match &self.backend {
            LanceDbBackend::InMemory { points } => {
                let points = points
                    .lock()
                    .map_err(|_| "lancedb points lock poisoned".to_string())?;
                let mut hits: Vec<_> = points
                    .iter()
                    .filter(|point| point.snapshot_id == snapshot_id)
                    .map(|point| VectorSearchHit {
                        snapshot_id: point.snapshot_id.clone(),
                        chunk_id: point.chunk_id.clone(),
                        symbol_path: point.symbol_path.clone(),
                        score: score_from_metric(self.distance_metric, &point.vector, query_vector),
                    })
                    .collect();
                hits.sort_by(|left, right| right.score.total_cmp(&left.score));
                hits.truncate(limit);
                Ok(hits)
            }
            LanceDbBackend::Local => {
                let connection = lancedb::connect(&self.db_root)
                    .execute()
                    .await
                    .map_err(|err| err.to_string())?;
                let table_names = connection
                    .table_names()
                    .execute()
                    .await
                    .map_err(|err| err.to_string())?;
                if !table_names.iter().any(|name| name == &self.table_name) {
                    return Ok(Vec::new());
                }

                let table = connection
                    .open_table(&self.table_name)
                    .execute()
                    .await
                    .map_err(|err| err.to_string())?;
                let batches = table
                    .query()
                    .only_if(snapshot_filter(snapshot_id))
                    .limit(limit)
                    .nearest_to(query_vector)
                    .map_err(|err| err.to_string())?
                    .distance_type(lancedb_distance_type(self.distance_metric))
                    .execute()
                    .await
                    .map_err(|err| err.to_string())?
                    .try_collect::<Vec<_>>()
                    .await
                    .map_err(|err| err.to_string())?;

                let mut hits = Vec::new();
                for batch in batches {
                    let chunk_ids = batch
                        .column_by_name("chunk_id")
                        .and_then(|column| column.as_any().downcast_ref::<StringArray>())
                        .ok_or_else(|| "lancedb result missing chunk_id column".to_string())?;
                    let snapshot_ids = batch
                        .column_by_name("snapshot_id")
                        .and_then(|column| column.as_any().downcast_ref::<StringArray>())
                        .ok_or_else(|| "lancedb result missing snapshot_id column".to_string())?;
                    let symbol_paths = batch
                        .column_by_name("symbol_path")
                        .and_then(|column| column.as_any().downcast_ref::<StringArray>());
                    let distances = distance_values(&batch)?;

                    for index in 0..batch.num_rows() {
                        let score = score_from_distance(self.distance_metric, distances[index]);
                        hits.push(VectorSearchHit {
                            snapshot_id: snapshot_ids.value(index).to_string(),
                            chunk_id: chunk_ids.value(index).to_string(),
                            symbol_path: symbol_paths.and_then(|values| {
                                if values.is_null(index) {
                                    None
                                } else {
                                    Some(values.value(index).to_string())
                                }
                            }),
                            score,
                        });
                    }
                }

                hits.sort_by(|left, right| right.score.total_cmp(&left.score));
                hits.truncate(limit);
                Ok(hits)
            }
        }
    }

    pub async fn point_count(&self) -> Result<usize, String> {
        match &self.backend {
            LanceDbBackend::InMemory { points } => points
                .lock()
                .map(|points| points.len())
                .map_err(|_| "lancedb points lock poisoned".to_string()),
            LanceDbBackend::Local => {
                let connection = lancedb::connect(&self.db_root)
                    .execute()
                    .await
                    .map_err(|err| err.to_string())?;
                let table_names = connection
                    .table_names()
                    .execute()
                    .await
                    .map_err(|err| err.to_string())?;
                if !table_names.iter().any(|name| name == &self.table_name) {
                    return Ok(0);
                }
                let table = connection
                    .open_table(&self.table_name)
                    .execute()
                    .await
                    .map_err(|err| err.to_string())?;
                table.count_rows(None).await.map_err(|err| err.to_string())
            }
        }
    }

    fn schema(&self) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("chunk_id", DataType::Utf8, false),
            Field::new("snapshot_id", DataType::Utf8, false),
            Field::new("symbol_path", DataType::Utf8, true),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    self.dimensions as i32,
                ),
                true,
            ),
        ]))
    }

    async fn ensure_table(&self) -> Result<lancedb::Table, String> {
        let connection = lancedb::connect(&self.db_root)
            .execute()
            .await
            .map_err(|err| err.to_string())?;

        let table_names = connection
            .table_names()
            .execute()
            .await
            .map_err(|err| err.to_string())?;

        if !table_names.iter().any(|name| name == &self.table_name) {
            connection
                .create_empty_table(&self.table_name, self.schema())
                .execute()
                .await
                .map_err(|err| err.to_string())?;
        }

        connection
            .open_table(&self.table_name)
            .execute()
            .await
            .map_err(|err| err.to_string())
    }

    async fn ensure_vector_index(&self, table: &lancedb::Table) -> Result<(), String> {
        table
            .create_index(
                &["vector"],
                Index::IvfFlat(
                    IvfFlatIndexBuilder::default()
                        .distance_type(lancedb_distance_type(self.distance_metric)),
                ),
            )
            .replace(true)
            .execute()
            .await
            .map_err(|err| err.to_string())
    }

    fn build_batch(
        &self,
        snapshot_id: &str,
        chunks: &[Chunk],
        vectors: Vec<Vec<f32>>,
    ) -> Result<RecordBatch, String> {
        let schema = self.schema();
        let chunk_ids: Vec<&str> = chunks.iter().map(|chunk| chunk.id.as_str()).collect();
        let snapshot_ids: Vec<&str> = std::iter::repeat_n(snapshot_id, chunks.len()).collect();
        let symbol_paths: Vec<Option<&str>> = chunks
            .iter()
            .map(|chunk| chunk.symbol_path.as_deref())
            .collect();
        let vector_array = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
            vectors
                .into_iter()
                .map(|vector| Some(vector.into_iter().map(Some).collect::<Vec<_>>())),
            self.dimensions as i32,
        );

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(chunk_ids)),
                Arc::new(StringArray::from(snapshot_ids)),
                Arc::new(StringArray::from(symbol_paths)),
                Arc::new(vector_array),
            ],
        )
        .map_err(|err| err.to_string())
    }
}

fn validate_vectors(
    table_name: &str,
    dimensions: usize,
    chunks: &[Chunk],
    vectors: &[Vec<f32>],
) -> Result<(), String> {
    if vectors.iter().any(|vector| vector.len() != dimensions) {
        return Err(format!(
            "vector dimensions did not match table {} dimensions {}",
            table_name, dimensions
        ));
    }
    if vectors.len() != chunks.len() {
        return Err("vector count did not match chunk count".to_string());
    }
    Ok(())
}

fn snapshot_filter(snapshot_id: &str) -> String {
    format!("snapshot_id = '{}'", snapshot_id.replace('\'', "''"))
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

fn l2_distance_squared(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(a, b)| {
            let delta = a - b;
            delta * delta
        })
        .sum()
}

fn dot_product(left: &[f32], right: &[f32]) -> f32 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn lancedb_distance_type(distance_metric: VectorDistanceMetric) -> lancedb::DistanceType {
    match distance_metric {
        VectorDistanceMetric::Cosine => lancedb::DistanceType::Cosine,
        VectorDistanceMetric::L2 => lancedb::DistanceType::L2,
        VectorDistanceMetric::Dot => lancedb::DistanceType::Dot,
    }
}

fn score_from_metric(distance_metric: VectorDistanceMetric, left: &[f32], right: &[f32]) -> f32 {
    match distance_metric {
        VectorDistanceMetric::Cosine => cosine_similarity(left, right),
        VectorDistanceMetric::L2 => -l2_distance_squared(left, right),
        VectorDistanceMetric::Dot => dot_product(left, right),
    }
}

fn score_from_distance(distance_metric: VectorDistanceMetric, distance: f32) -> f32 {
    match distance_metric {
        VectorDistanceMetric::Cosine => 1.0 - distance,
        VectorDistanceMetric::L2 => -distance,
        VectorDistanceMetric::Dot => 1.0 - distance,
    }
}

fn distance_values(batch: &RecordBatch) -> Result<Vec<f32>, String> {
    let distances = batch
        .column_by_name("_distance")
        .ok_or_else(|| "lancedb result missing _distance column".to_string())?;

    if let Some(values) = distances.as_any().downcast_ref::<Float32Array>() {
        return (0..values.len())
            .map(|index| {
                if values.is_null(index) {
                    Err("lancedb result contains null _distance".to_string())
                } else if !values.value(index).is_finite() {
                    Err("lancedb result contains non-finite _distance".to_string())
                } else {
                    Ok(values.value(index))
                }
            })
            .collect();
    }

    if let Some(values) = distances.as_any().downcast_ref::<Float64Array>() {
        return (0..values.len())
            .map(|index| {
                if values.is_null(index) {
                    Err("lancedb result contains null _distance".to_string())
                } else if !values.value(index).is_finite() {
                    Err("lancedb result contains non-finite _distance".to_string())
                } else {
                    Ok(values.value(index) as f32)
                }
            })
            .collect();
    }

    Err(format!(
        "lancedb _distance column had unsupported type {}",
        distances.data_type()
    ))
}

#[cfg(test)]
mod tests {
    use super::{distance_values, LanceDbPointStore};
    use crate::chunking::{Chunk, ChunkKind, SourceSpan};
    use crate::config::VectorDistanceMetric;
    use std::sync::Arc;

    use arrow_array::{Float32Array, Float64Array, Int32Array, RecordBatch};
    use arrow_schema::{DataType, Field, Schema};
    use tempfile::tempdir;

    fn sample_chunks() -> Vec<Chunk> {
        vec![
            Chunk {
                id: "chunk-1".to_string(),
                kind: ChunkKind::Symbol,
                file_path: "src/lib.rs".into(),
                span: SourceSpan {
                    start_byte: 0,
                    end_byte: 10,
                },
                symbol_path: Some("mini_repo::a".to_string()),
                symbol_name: Some("a".to_string()),
                owning_symbol_header: None,
                docs_text: None,
                signature_text: None,
                parent_symbol_path: None,
                retrieval_markers: Vec::new(),
                repository_state_hints: Vec::new(),
                text: "fn a() {}".to_string(),
            },
            Chunk {
                id: "chunk-2".to_string(),
                kind: ChunkKind::Symbol,
                file_path: "src/lib.rs".into(),
                span: SourceSpan {
                    start_byte: 11,
                    end_byte: 21,
                },
                symbol_path: Some("mini_repo::b".to_string()),
                symbol_name: Some("b".to_string()),
                owning_symbol_header: None,
                docs_text: None,
                signature_text: None,
                parent_symbol_path: None,
                retrieval_markers: Vec::new(),
                repository_state_hints: Vec::new(),
                text: "fn b() {}".to_string(),
            },
        ]
    }

    #[tokio::test]
    async fn rejects_dimension_mismatch() {
        let store = LanceDbPointStore::new_in_memory("memory://test", "vectors", 4);
        let err = store
            .replace_snapshot("snap-1", &sample_chunks(), vec![vec![1.0, 0.0, 0.0]])
            .await
            .expect_err("dimension mismatch should fail");

        assert!(err.contains("vector dimensions"), "unexpected err: {err}");
    }

    #[tokio::test]
    async fn replaces_snapshot_rows_idempotently() {
        let store = LanceDbPointStore::new_in_memory("memory://test", "vectors", 4);
        let chunks = sample_chunks();

        let inserted = store
            .replace_snapshot(
                "snap-1",
                &chunks,
                vec![vec![1.0, 0.0, 0.0, 0.0], vec![0.0, 1.0, 0.0, 0.0]],
            )
            .await
            .expect("first insert");
        assert_eq!(inserted, 2);

        let replaced = store
            .replace_snapshot("snap-1", &chunks[..1], vec![vec![1.0, 0.0, 0.0, 0.0]])
            .await
            .expect("replace snapshot");
        assert_eq!(replaced, 1);

        assert_eq!(store.point_count().await.expect("point count"), 1);
    }

    #[tokio::test]
    async fn returns_sorted_hits_with_limit() {
        let store = LanceDbPointStore::new_in_memory_with_metric(
            "memory://test",
            "vectors",
            3,
            VectorDistanceMetric::Cosine,
        );
        let chunks = sample_chunks();
        store
            .replace_snapshot(
                "snap-1",
                &chunks,
                vec![vec![1.0, 0.0, 0.0], vec![0.2, 0.0, 0.0]],
            )
            .await
            .expect("insert vectors");

        let hits = store
            .search_snapshot("snap-1", &[1.0, 0.0, 0.0], 1)
            .await
            .expect("search");

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].chunk_id, "chunk-1");
    }

    #[test]
    fn extracts_distance_values_from_float32_and_float64() {
        let schema = Arc::new(Schema::new(vec![Field::new(
            "_distance",
            DataType::Float32,
            false,
        )]));
        let batch = RecordBatch::try_new(schema, vec![Arc::new(Float32Array::from(vec![0.2]))])
            .expect("f32 batch");
        let values = distance_values(&batch).expect("f32 parse");
        assert_eq!(values, vec![0.2]);

        let schema = Arc::new(Schema::new(vec![Field::new(
            "_distance",
            DataType::Float64,
            false,
        )]));
        let batch = RecordBatch::try_new(schema, vec![Arc::new(Float64Array::from(vec![0.4]))])
            .expect("f64 batch");
        let values = distance_values(&batch).expect("f64 parse");
        assert_eq!(values, vec![0.4]);
    }

    #[test]
    fn rejects_missing_or_unsupported_distance_column() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        let batch = RecordBatch::try_new(schema, vec![Arc::new(Int32Array::from(vec![1]))])
            .expect("int batch");
        let err = distance_values(&batch).expect_err("missing _distance should fail");
        assert!(err.contains("_distance"), "unexpected err: {err}");

        let schema = Arc::new(Schema::new(vec![Field::new(
            "_distance",
            DataType::Int32,
            false,
        )]));
        let batch = RecordBatch::try_new(schema, vec![Arc::new(Int32Array::from(vec![1]))])
            .expect("int distance batch");
        let err = distance_values(&batch).expect_err("unsupported distance type should fail");
        assert!(err.contains("unsupported type"), "unexpected err: {err}");
    }

    #[tokio::test]
    async fn local_and_memory_scores_are_metric_consistent() {
        let chunks = sample_chunks();
        let vectors = vec![vec![0.8, 0.2, 0.0], vec![0.1, 0.9, 0.0]];
        let query = vec![0.9, 0.1, 0.0];
        let metrics = [
            VectorDistanceMetric::Cosine,
            VectorDistanceMetric::L2,
            VectorDistanceMetric::Dot,
        ];

        for metric in metrics {
            let dir = tempdir().expect("tempdir");
            let local = LanceDbPointStore::new_with_metric(
                dir.path().display().to_string(),
                format!("vectors_{metric:?}"),
                3,
                metric,
            )
            .expect("local store");
            let memory = LanceDbPointStore::new_in_memory_with_metric(
                "memory://parity",
                format!("vectors_{metric:?}"),
                3,
                metric,
            );

            local
                .replace_snapshot("snap-1", &chunks, vectors.clone())
                .await
                .expect("local insert");
            memory
                .replace_snapshot("snap-1", &chunks, vectors.clone())
                .await
                .expect("memory insert");

            let local_hits = local
                .search_snapshot("snap-1", &query, 2)
                .await
                .expect("local search");
            let memory_hits = memory
                .search_snapshot("snap-1", &query, 2)
                .await
                .expect("memory search");

            assert_eq!(local_hits.len(), memory_hits.len(), "metric={metric:?}");
            assert_eq!(
                local_hits[0].chunk_id, memory_hits[0].chunk_id,
                "metric={metric:?}"
            );
            let delta = (local_hits[0].score - memory_hits[0].score).abs();
            assert!(
                delta < 0.001,
                "metric={metric:?} score mismatch local={} memory={} delta={delta}",
                local_hits[0].score,
                memory_hits[0].score
            );
        }
    }
}
