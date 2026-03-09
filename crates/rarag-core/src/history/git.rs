#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitChangeSummary {
    pub status: String,
    pub from_path: Option<String>,
    pub to_path: Option<String>,
}

pub fn parse_name_status_rename_chain(lines: &[String]) -> Vec<String> {
    let mut chain = Vec::new();
    for line in lines {
        let mut parts = line.split('\t');
        let status = parts.next().unwrap_or_default();
        if !status.starts_with('R') {
            continue;
        }
        let Some(from) = parts.next() else {
            continue;
        };
        let Some(to) = parts.next() else {
            continue;
        };
        if chain.is_empty() {
            chain.push(from.to_string());
        }
        if chain.last().is_some_and(|last| last == from) {
            chain.push(to.to_string());
        }
    }
    chain
}
