use crate::metadata::{HistoryNodeRecord, LineageEdgeRecord};

pub fn derive_lineage_edges(
    snapshot_id: &str,
    nodes: &[HistoryNodeRecord],
) -> Vec<LineageEdgeRecord> {
    let mut edges = Vec::new();
    for window in nodes.windows(2) {
        let left = &window[0];
        let right = &window[1];
        let summary = format!(
            "{} {}",
            left.summary.to_lowercase(),
            right.summary.to_lowercase()
        );
        if summary.contains("fix") || summary.contains("bug") {
            edges.push(LineageEdgeRecord::new(
                format!("lineage:{}:{}", left.node_id, right.node_id),
                snapshot_id.to_string(),
                left.node_id.clone(),
                right.node_id.clone(),
                "fixes",
                Some("heuristic message keyword".to_string()),
                0.55,
            ));
        }
        if left.subject.is_some() && right.subject.is_some() {
            edges.push(LineageEdgeRecord::new(
                format!("lineage-followed:{}:{}", left.node_id, right.node_id),
                snapshot_id.to_string(),
                left.node_id.clone(),
                right.node_id.clone(),
                "followed_by",
                Some("commit sequence".to_string()),
                0.8,
            ));
        }
    }
    edges
}
