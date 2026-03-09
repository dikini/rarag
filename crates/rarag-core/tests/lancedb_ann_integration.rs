use std::collections::HashSet;
use std::sync::Arc;

use arrow_array::types::Float32Type;
use arrow_array::{Array, FixedSizeListArray, Float32Array, Float64Array, RecordBatch, RecordBatchIterator, StringArray};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::index::vector::IvfFlatIndexBuilder;
use lancedb::index::Index;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{connect, DistanceType};
use tempfile::tempdir;
use tokio::runtime::Runtime;

fn runtime() -> Runtime {
    Runtime::new().expect("tokio runtime")
}

#[derive(Debug, Clone)]
struct SearchHit {
    chunk_id: String,
    score: f32,
}

#[test]
fn ann_score_drift_stays_bounded_vs_flat_baseline() {
    runtime().block_on(async {
        let metrics = [DistanceType::Cosine, DistanceType::L2, DistanceType::Dot];
        for metric in metrics {
            run_metric_case(metric).await;
        }
    });
}

async fn run_metric_case(metric: DistanceType) {
    let dir = tempdir().expect("tempdir");
    let connection = connect(dir.path().to_str().expect("db path"))
        .execute()
        .await
        .expect("connect lancedb");

    let dims = 16usize;
    let rows = 384usize;
    let top_k = 10usize;
    let table_name = format!("ann_{metric:?}").to_lowercase();
    let vectors: Vec<Vec<f32>> = (0..rows).map(|i| synthetic_vector(i, dims)).collect();
    let query = tweak_query_vector(&vectors[123]);
    let batch = make_batch(&vectors, "snap-ann");

    let table = connection
        .create_table(
            &table_name,
            Box::new(RecordBatchIterator::new(
                vec![Ok(batch)].into_iter(),
                schema(dims),
            )),
        )
        .execute()
        .await
        .expect("create table");

    table
        .create_index(
            &["vector"],
            Index::IvfFlat(IvfFlatIndexBuilder::default().distance_type(metric)),
        )
        .replace(true)
        .execute()
        .await
        .expect("create vector index");

    let baseline = query_hits(&table, "snap-ann", &query, top_k, metric, None, None, true).await;
    assert_eq!(baseline.len(), top_k, "metric={metric:?}");

    let configs = [(8usize, None), (8usize, Some(2u32)), (20usize, Some(4u32))];
    for (nprobes, refine_factor) in configs {
        let ann = query_hits(
            &table,
            "snap-ann",
            &query,
            top_k,
            metric,
            Some(nprobes),
            refine_factor,
            false,
        )
        .await;
        assert_eq!(ann.len(), top_k, "metric={metric:?} nprobes={nprobes}");

        let overlap = topk_overlap(&baseline, &ann);
        assert!(
            overlap >= 0.8,
            "metric={metric:?} nprobes={nprobes} refine={refine_factor:?} overlap={overlap}"
        );

        let baseline_top1 = &baseline[0];
        let ann_top1 = &ann[0];
        assert_eq!(
            baseline_top1.chunk_id, ann_top1.chunk_id,
            "metric={metric:?} nprobes={nprobes} refine={refine_factor:?}"
        );
        let score_delta = (baseline_top1.score - ann_top1.score).abs();
        assert!(
            score_delta < 0.25,
            "metric={metric:?} nprobes={nprobes} refine={refine_factor:?} score_delta={score_delta} baseline={} ann={}",
            baseline_top1.score,
            ann_top1.score
        );
    }
}

async fn query_hits(
    table: &lancedb::Table,
    snapshot_id: &str,
    query: &[f32],
    top_k: usize,
    metric: DistanceType,
    nprobes: Option<usize>,
    refine_factor: Option<u32>,
    bypass_vector_index: bool,
) -> Vec<SearchHit> {
    let mut query_builder = table
        .query()
        .only_if(format!("snapshot_id = '{}'", snapshot_id))
        .limit(top_k)
        .nearest_to(query)
        .expect("nearest_to")
        .distance_type(metric);
    if let Some(value) = nprobes {
        query_builder = query_builder.nprobes(value);
    }
    if let Some(value) = refine_factor {
        query_builder = query_builder.refine_factor(value);
    }
    if bypass_vector_index {
        query_builder = query_builder.bypass_vector_index();
    }

    let batches = query_builder
        .execute()
        .await
        .expect("execute query")
        .try_collect::<Vec<_>>()
        .await
        .expect("collect batches");

    let mut hits = Vec::new();
    for batch in batches {
        let chunk_ids = batch
            .column_by_name("chunk_id")
            .and_then(|column| column.as_any().downcast_ref::<StringArray>())
            .expect("chunk_id column");
        let distances = extract_distances(&batch);

        for index in 0..batch.num_rows() {
            hits.push(SearchHit {
                chunk_id: chunk_ids.value(index).to_string(),
                score: score_from_distance(metric, distances[index]),
            });
        }
    }
    hits.sort_by(|left, right| right.score.total_cmp(&left.score));
    hits.truncate(top_k);
    hits
}

fn extract_distances(batch: &RecordBatch) -> Vec<f32> {
    let column = batch.column_by_name("_distance").expect("_distance column");
    if let Some(values) = column.as_any().downcast_ref::<Float32Array>() {
        return (0..values.len()).map(|index| values.value(index)).collect();
    }
    if let Some(values) = column.as_any().downcast_ref::<Float64Array>() {
        return (0..values.len())
            .map(|index| values.value(index) as f32)
            .collect();
    }
    panic!("unsupported _distance type: {}", column.data_type());
}

fn score_from_distance(metric: DistanceType, distance: f32) -> f32 {
    match metric {
        DistanceType::Cosine => 1.0 - distance,
        DistanceType::L2 => -distance,
        DistanceType::Dot => 1.0 - distance,
        DistanceType::Hamming => -distance,
        _ => panic!("unsupported metric in test score mapping: {metric:?}"),
    }
}

fn topk_overlap(left: &[SearchHit], right: &[SearchHit]) -> f32 {
    let left_set: HashSet<_> = left.iter().map(|hit| hit.chunk_id.as_str()).collect();
    let right_set: HashSet<_> = right.iter().map(|hit| hit.chunk_id.as_str()).collect();
    let common = left_set.intersection(&right_set).count();
    common as f32 / left.len() as f32
}

fn schema(dims: usize) -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("chunk_id", DataType::Utf8, false),
        Field::new("snapshot_id", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                dims as i32,
            ),
            true,
        ),
    ]))
}

fn make_batch(vectors: &[Vec<f32>], snapshot_id: &str) -> RecordBatch {
    let dims = vectors.first().map(Vec::len).unwrap_or_default();
    let chunk_ids: Vec<String> = (0..vectors.len())
        .map(|index| format!("chunk-{index:04}"))
        .collect();
    let snapshot_ids: Vec<&str> = std::iter::repeat_n(snapshot_id, vectors.len()).collect();
    let vector_array = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
        vectors
            .iter()
            .map(|vector| Some(vector.iter().copied().map(Some).collect::<Vec<_>>())),
        dims as i32,
    );

    RecordBatch::try_new(
        schema(dims),
        vec![
            Arc::new(StringArray::from(chunk_ids)),
            Arc::new(StringArray::from(snapshot_ids)),
            Arc::new(vector_array),
        ],
    )
    .expect("record batch")
}

fn synthetic_vector(seed: usize, dims: usize) -> Vec<f32> {
    (0..dims)
        .map(|index| {
            let x = (seed as f32 * 0.37) + (index as f32 * 0.19);
            (x.sin() + x.cos() * 0.5) * 0.5
        })
        .collect()
}

fn tweak_query_vector(base: &[f32]) -> Vec<f32> {
    base.iter()
        .enumerate()
        .map(|(index, value)| value + ((index as f32) * 0.001))
        .collect()
}
