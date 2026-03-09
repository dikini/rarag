use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvRowChunk {
    pub id: String,
    pub file_path: String,
    pub document_kind: String,
    pub row_number: u32,
    pub text: String,
}

pub fn chunk_csv_rows(
    file_path: &Path,
    body: &str,
    document_kind: &str,
) -> Result<Vec<CsvRowChunk>, String> {
    let mut rows = body.lines();
    let Some(header_line) = rows.next() else {
        return Ok(Vec::new());
    };
    let headers = parse_csv_line(header_line);
    if headers.is_empty() {
        return Ok(Vec::new());
    }
    let file_display = file_path.display().to_string();
    let mut chunks = Vec::new();
    for (index, row) in rows.enumerate() {
        if row.trim().is_empty() {
            continue;
        }
        let values = parse_csv_line(row);
        if values.is_empty() || values.iter().all(|value| value.trim().is_empty()) {
            continue;
        }
        let mut pairs = Vec::new();
        for (column_index, key) in headers.iter().enumerate() {
            let value = values.get(column_index).cloned().unwrap_or_default();
            pairs.push(format!("{key}: {value}"));
        }
        let row_number = u32::try_from(index + 2).map_err(|err| err.to_string())?;
        chunks.push(CsvRowChunk {
            id: format!("{file_display}#row-{row_number}"),
            file_path: file_display.clone(),
            document_kind: document_kind.to_string(),
            row_number,
            text: pairs.join(" | "),
        });
    }

    Ok(chunks)
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let chars = line.chars().peekable();
    for ch in chars {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                out.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    out.push(current.trim().to_string());
    out
}
