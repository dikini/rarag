use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownChunk {
    pub id: String,
    pub file_path: String,
    pub document_kind: String,
    pub heading_path: Vec<String>,
    pub start_line: u32,
    pub end_line: u32,
    pub text: String,
}

pub fn chunk_markdown(
    file_path: &Path,
    body: &str,
    document_kind: &str,
) -> Result<Vec<MarkdownChunk>, String> {
    let file_display = file_path.display().to_string();
    let lines: Vec<&str> = body.lines().collect();
    if lines.is_empty() {
        return Ok(Vec::new());
    }

    let mut chunks = Vec::new();
    let mut heading_stack: Vec<(usize, String)> = Vec::new();
    let mut section_start = 1_usize;
    let mut section_heading = String::new();

    let flush_section = |chunks: &mut Vec<MarkdownChunk>,
                         start: usize,
                         end: usize,
                         heading_stack: &[(usize, String)],
                         section_heading: &str|
     -> Result<(), String> {
        if start > end || end == 0 {
            return Ok(());
        }
        let mut text = String::new();
        for (index, line) in lines[start - 1..end].iter().enumerate() {
            if index > 0 {
                text.push('\n');
            }
            text.push_str(line);
        }
        if text.trim().is_empty() {
            return Ok(());
        }
        let heading_path = heading_stack
            .iter()
            .map(|(_, value)| value.clone())
            .collect::<Vec<_>>();
        let heading_label = if heading_path.is_empty() {
            "root".to_string()
        } else {
            heading_path.join(" > ")
        };
        let id = format!("{file_display}#{}:{start}-{end}", heading_label);
        chunks.push(MarkdownChunk {
            id,
            file_path: file_display.clone(),
            document_kind: document_kind.to_string(),
            heading_path: if heading_path.is_empty() && !section_heading.is_empty() {
                vec![section_heading.to_string()]
            } else {
                heading_path
            },
            start_line: u32::try_from(start).map_err(|err| err.to_string())?,
            end_line: u32::try_from(end).map_err(|err| err.to_string())?,
            text,
        });
        Ok(())
    };

    for (idx, line) in lines.iter().enumerate() {
        let line_number = idx + 1;
        let trimmed = line.trim_start();
        let heading_level = trimmed.chars().take_while(|ch| *ch == '#').count().min(6);
        let heading_title = trimmed
            .strip_prefix('#')
            .map(str::trim)
            .filter(|_| heading_level > 0)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);

        if let Some(heading_title) = heading_title {
            flush_section(
                &mut chunks,
                section_start,
                line_number.saturating_sub(1),
                &heading_stack,
                &section_heading,
            )?;
            while heading_stack
                .last()
                .is_some_and(|(existing_level, _)| *existing_level >= heading_level)
            {
                heading_stack.pop();
            }
            heading_stack.push((heading_level, heading_title.clone()));
            section_start = line_number;
            section_heading = heading_title;
        }
    }

    flush_section(
        &mut chunks,
        section_start,
        lines.len(),
        &heading_stack,
        &section_heading,
    )?;
    Ok(chunks)
}
