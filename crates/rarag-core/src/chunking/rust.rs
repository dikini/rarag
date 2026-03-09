use std::fs;
use std::path::{Path, PathBuf};

use ra_ap_syntax::{
    AstNode, Edition, SourceFile, TextRange, TextSize,
    ast::{self, HasAttrs, HasModuleItem, HasName},
};

use crate::chunking::types::{Chunk, ChunkKind, SourceSpan};
use crate::chunking::{chunk_csv_rows, chunk_markdown};
use crate::config::{DocumentSourceParser, DocumentSourceRule, DocumentSourcesConfig};

#[derive(Debug, Clone)]
pub struct RustChunker {
    max_body_bytes: usize,
    document_sources: DocumentSourcesConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceRootKind {
    Src,
    Examples,
    IntegrationTests,
}

#[derive(Debug, Clone)]
struct SourceRoot {
    kind: SourceRootKind,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct DocBundle {
    text: String,
    range: TextRange,
}

#[derive(Debug, Clone, Default)]
struct ChunkMetadata {
    symbol_path: Option<String>,
    symbol_name: Option<String>,
    owning_symbol_header: Option<String>,
    docs_text: Option<String>,
    signature_text: Option<String>,
    parent_symbol_path: Option<String>,
    retrieval_markers: Vec<String>,
    repository_state_hints: Vec<String>,
    text_override: Option<String>,
    id_suffix: Option<String>,
}

impl RustChunker {
    pub fn new(max_body_bytes: usize) -> Self {
        Self {
            max_body_bytes,
            document_sources: DocumentSourcesConfig::default(),
        }
    }

    pub fn new_with_document_sources(
        max_body_bytes: usize,
        document_sources: DocumentSourcesConfig,
    ) -> Self {
        Self {
            max_body_bytes,
            document_sources,
        }
    }

    pub fn chunk_workspace(&self, root: &Path) -> Result<Vec<Chunk>, String> {
        let crate_name = crate_name(root)?;
        let mut chunks = Vec::new();

        for source_root in source_roots(root) {
            let mut rust_files = Vec::new();
            collect_rust_files(&source_root.path, &mut rust_files)?;
            rust_files.sort();

            for file_path in rust_files {
                let relative = file_path
                    .strip_prefix(&source_root.path)
                    .map_err(|err| err.to_string())?;
                let module_path = module_path_for_file(&crate_name, relative, source_root.kind);
                let source = fs::read_to_string(&file_path).map_err(|err| err.to_string())?;
                let parse = SourceFile::parse(&source, Edition::CURRENT);
                let source_file = parse.tree();
                let items: Vec<_> = source_file.items().collect();

                chunks.push(file_chunk(
                    &file_path,
                    &source,
                    &module_path,
                    source_root.kind,
                ));
                collect_items(
                    &source,
                    &file_path,
                    &module_path,
                    items,
                    self.max_body_bytes,
                    source_root.kind,
                    &mut chunks,
                )?;
            }
        }
        collect_document_chunks(root, &crate_name, &self.document_sources, &mut chunks)?;

        Ok(chunks)
    }
}

fn source_roots(root: &Path) -> Vec<SourceRoot> {
    [
        (SourceRootKind::Src, root.join("src")),
        (SourceRootKind::Examples, root.join("examples")),
        (SourceRootKind::IntegrationTests, root.join("tests")),
    ]
    .into_iter()
    .filter(|(_, path)| path.exists())
    .map(|(kind, path)| SourceRoot { kind, path })
    .collect()
}

fn collect_items(
    source: &str,
    file_path: &Path,
    module_path: &str,
    items: Vec<ast::Item>,
    max_body_bytes: usize,
    source_root_kind: SourceRootKind,
    chunks: &mut Vec<Chunk>,
) -> Result<(), String> {
    for item in items {
        match item {
            ast::Item::Fn(function) => {
                let name = item_name(&function)?;
                let symbol_path = format!("{module_path}::{name}");
                let syntax = function.syntax();
                let body_range = function.body().map(|body| body.syntax().text_range());
                let signature_text = item_signature(source, syntax.text_range(), body_range);
                let doc_bundle = doc_bundle_from_source(source, syntax.text_range());
                let docs_text = doc_bundle.as_ref().map(|bundle| bundle.text.clone());
                let mut retrieval_markers = root_markers(source_root_kind);
                let mut repository_state_hints = root_state_hints(source_root_kind);
                if function.has_atom_attr("test")
                    && !retrieval_markers.iter().any(|item| item == "test")
                {
                    retrieval_markers.push("test".to_string());
                    repository_state_hints.push("tests".to_string());
                }

                let metadata = ChunkMetadata {
                    symbol_path: Some(symbol_path.clone()),
                    symbol_name: Some(name.clone()),
                    docs_text: docs_text.clone(),
                    signature_text: Some(signature_text.clone()),
                    parent_symbol_path: Some(module_path.to_string()),
                    retrieval_markers: retrieval_markers.clone(),
                    repository_state_hints: repository_state_hints.clone(),
                    ..ChunkMetadata::default()
                };
                chunks.push(chunk_from_range(
                    file_path,
                    source,
                    ChunkKind::Symbol,
                    syntax.text_range(),
                    metadata,
                ));

                if let Some(body_range) = body_range
                    && span_len(body_range) > max_body_bytes
                {
                    chunks.push(chunk_from_range(
                        file_path,
                        source,
                        ChunkKind::BodyRegion,
                        body_range,
                        ChunkMetadata {
                            symbol_path: Some(symbol_path.clone()),
                            symbol_name: Some(name.clone()),
                            owning_symbol_header: Some(signature_text.clone()),
                            signature_text: Some(signature_text.clone()),
                            parent_symbol_path: Some(symbol_path.clone()),
                            retrieval_markers: retrieval_markers.clone(),
                            repository_state_hints: repository_state_hints.clone(),
                            ..ChunkMetadata::default()
                        },
                    ));
                }

                if function.has_atom_attr("test") {
                    chunks.push(chunk_from_range(
                        file_path,
                        source,
                        ChunkKind::TestFunction,
                        syntax.text_range(),
                        ChunkMetadata {
                            symbol_path: Some(symbol_path.clone()),
                            symbol_name: Some(name.clone()),
                            signature_text: Some(signature_text.clone()),
                            parent_symbol_path: Some(module_path.to_string()),
                            retrieval_markers: vec!["test".to_string()],
                            repository_state_hints: vec!["tests".to_string()],
                            ..ChunkMetadata::default()
                        },
                    ));
                }

                if let Some(doc_bundle) = doc_bundle {
                    append_doctest_chunks(
                        file_path,
                        source,
                        &symbol_path,
                        &name,
                        &signature_text,
                        &doc_bundle,
                        chunks,
                    );
                }
            }
            ast::Item::Struct(strukt) => {
                let name = item_name(&strukt)?;
                let symbol_path = format!("{module_path}::{name}");
                let doc_bundle = doc_bundle_from_source(source, strukt.syntax().text_range());
                let signature_text = item_signature(source, strukt.syntax().text_range(), None);
                chunks.push(chunk_from_range(
                    file_path,
                    source,
                    ChunkKind::Symbol,
                    strukt.syntax().text_range(),
                    ChunkMetadata {
                        symbol_path: Some(symbol_path.clone()),
                        symbol_name: Some(name.clone()),
                        docs_text: doc_bundle.as_ref().map(|bundle| bundle.text.clone()),
                        signature_text: Some(signature_text.clone()),
                        parent_symbol_path: Some(module_path.to_string()),
                        retrieval_markers: root_markers(source_root_kind),
                        repository_state_hints: root_state_hints(source_root_kind),
                        ..ChunkMetadata::default()
                    },
                ));
                if let Some(doc_bundle) = doc_bundle {
                    append_doctest_chunks(
                        file_path,
                        source,
                        &symbol_path,
                        &name,
                        &signature_text,
                        &doc_bundle,
                        chunks,
                    );
                }
            }
            ast::Item::Module(module) => {
                let name = item_name(&module)?;
                let nested_path = format!("{module_path}::{name}");
                let doc_bundle = doc_bundle_from_source(source, module.syntax().text_range());
                let signature_text = item_signature(source, module.syntax().text_range(), None);
                chunks.push(chunk_from_range(
                    file_path,
                    source,
                    ChunkKind::ModuleSummary,
                    module.syntax().text_range(),
                    ChunkMetadata {
                        symbol_path: Some(nested_path.clone()),
                        symbol_name: Some(name.clone()),
                        docs_text: doc_bundle.as_ref().map(|bundle| bundle.text.clone()),
                        signature_text: Some(signature_text.clone()),
                        parent_symbol_path: Some(module_path.to_string()),
                        retrieval_markers: root_markers(source_root_kind),
                        repository_state_hints: root_state_hints(source_root_kind),
                        ..ChunkMetadata::default()
                    },
                ));
                if let Some(doc_bundle) = doc_bundle {
                    append_doctest_chunks(
                        file_path,
                        source,
                        &nested_path,
                        &name,
                        &signature_text,
                        &doc_bundle,
                        chunks,
                    );
                }
                if let Some(item_list) = module.item_list() {
                    collect_items(
                        source,
                        file_path,
                        &nested_path,
                        item_list.items().collect(),
                        max_body_bytes,
                        source_root_kind,
                        chunks,
                    )?;
                }
            }
            ast::Item::Enum(enm) => {
                let name = item_name(&enm)?;
                let symbol_path = format!("{module_path}::{name}");
                let doc_bundle = doc_bundle_from_source(source, enm.syntax().text_range());
                let signature_text = item_signature(source, enm.syntax().text_range(), None);
                chunks.push(chunk_from_range(
                    file_path,
                    source,
                    ChunkKind::Symbol,
                    enm.syntax().text_range(),
                    ChunkMetadata {
                        symbol_path: Some(symbol_path.clone()),
                        symbol_name: Some(name.clone()),
                        docs_text: doc_bundle.as_ref().map(|bundle| bundle.text.clone()),
                        signature_text: Some(signature_text.clone()),
                        parent_symbol_path: Some(module_path.to_string()),
                        retrieval_markers: root_markers(source_root_kind),
                        repository_state_hints: root_state_hints(source_root_kind),
                        ..ChunkMetadata::default()
                    },
                ));
                if let Some(doc_bundle) = doc_bundle {
                    append_doctest_chunks(
                        file_path,
                        source,
                        &symbol_path,
                        &name,
                        &signature_text,
                        &doc_bundle,
                        chunks,
                    );
                }
            }
            ast::Item::Trait(trait_item) => {
                let name = item_name(&trait_item)?;
                let symbol_path = format!("{module_path}::{name}");
                let doc_bundle = doc_bundle_from_source(source, trait_item.syntax().text_range());
                let signature_text = item_signature(source, trait_item.syntax().text_range(), None);
                chunks.push(chunk_from_range(
                    file_path,
                    source,
                    ChunkKind::Symbol,
                    trait_item.syntax().text_range(),
                    ChunkMetadata {
                        symbol_path: Some(symbol_path.clone()),
                        symbol_name: Some(name.clone()),
                        docs_text: doc_bundle.as_ref().map(|bundle| bundle.text.clone()),
                        signature_text: Some(signature_text.clone()),
                        parent_symbol_path: Some(module_path.to_string()),
                        retrieval_markers: root_markers(source_root_kind),
                        repository_state_hints: root_state_hints(source_root_kind),
                        ..ChunkMetadata::default()
                    },
                ));
                if let Some(doc_bundle) = doc_bundle {
                    append_doctest_chunks(
                        file_path,
                        source,
                        &symbol_path,
                        &name,
                        &signature_text,
                        &doc_bundle,
                        chunks,
                    );
                }
            }
            ast::Item::Impl(imp) => {
                let impl_symbol = impl_symbol_path(source, module_path, imp.syntax().text_range());
                let signature_text = item_signature(source, imp.syntax().text_range(), None);
                chunks.push(chunk_from_range(
                    file_path,
                    source,
                    ChunkKind::Symbol,
                    imp.syntax().text_range(),
                    ChunkMetadata {
                        symbol_path: Some(impl_symbol),
                        symbol_name: Some("impl".to_string()),
                        signature_text: Some(signature_text),
                        parent_symbol_path: Some(module_path.to_string()),
                        retrieval_markers: root_markers(source_root_kind),
                        repository_state_hints: root_state_hints(source_root_kind),
                        ..ChunkMetadata::default()
                    },
                ));
            }
            _ => {}
        }
    }

    Ok(())
}

fn append_doctest_chunks(
    file_path: &Path,
    source: &str,
    symbol_path: &str,
    symbol_name: &str,
    signature_text: &str,
    doc_bundle: &DocBundle,
    chunks: &mut Vec<Chunk>,
) {
    for (index, code) in extract_rust_doctests(&doc_bundle.text)
        .into_iter()
        .enumerate()
    {
        chunks.push(chunk_from_range(
            file_path,
            source,
            ChunkKind::Doctest,
            doc_bundle.range,
            ChunkMetadata {
                symbol_path: Some(symbol_path.to_string()),
                symbol_name: Some(symbol_name.to_string()),
                owning_symbol_header: Some(signature_text.to_string()),
                docs_text: Some(doc_bundle.text.clone()),
                signature_text: Some(signature_text.to_string()),
                parent_symbol_path: Some(symbol_path.to_string()),
                retrieval_markers: vec!["doctest".to_string(), "example".to_string()],
                repository_state_hints: vec!["tests".to_string(), "examples".to_string()],
                text_override: Some(code),
                id_suffix: Some(format!("doctest-{index}")),
            },
        ));
    }
}

fn doc_bundle_from_source(source: &str, item_range: TextRange) -> Option<DocBundle> {
    let item_start = u32::from(item_range.start()) as usize;
    let item_end = u32::from(item_range.end()) as usize;
    let item_text = &source[item_start..item_end];
    let mut lines = Vec::new();
    let mut doc_end = item_start;

    for segment in item_text.split_inclusive('\n') {
        let line = segment.trim_end_matches('\n');
        let trimmed = line.trim_start();
        if let Some(stripped) = trimmed.strip_prefix("///") {
            lines.push(stripped.trim_start().to_string());
            doc_end += segment.len();
            continue;
        }
        if let Some(stripped) = trimmed.strip_prefix("//!") {
            lines.push(stripped.trim_start().to_string());
            doc_end += segment.len();
            continue;
        }
        if trimmed.is_empty() && !lines.is_empty() {
            lines.push(String::new());
            doc_end += segment.len();
            continue;
        }
        break;
    }

    if lines.is_empty() {
        return None;
    }

    Some(DocBundle {
        text: lines.join("\n"),
        range: TextRange::new(
            TextSize::from(item_start as u32),
            TextSize::from(doc_end as u32),
        ),
    })
}

fn extract_rust_doctests(docs_text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    let mut in_rust_block = false;

    for line in docs_text.lines() {
        let trimmed = line.trim();
        if !in_rust_block {
            if let Some(fence) = trimmed.strip_prefix("```") {
                let language = fence.trim();
                if language.is_empty() || language.split(',').any(|part| part.trim() == "rust") {
                    in_rust_block = true;
                    current.clear();
                }
            }
            continue;
        }

        if trimmed == "```" {
            let block = current.join("\n").trim().to_string();
            if !block.is_empty() {
                blocks.push(block);
            }
            current.clear();
            in_rust_block = false;
            continue;
        }

        current.push(line.to_string());
    }

    blocks
}

fn file_chunk(
    file_path: &Path,
    source: &str,
    module_path: &str,
    root_kind: SourceRootKind,
) -> Chunk {
    let kind = match root_kind {
        SourceRootKind::Examples => ChunkKind::ExampleFile,
        _ => ChunkKind::CrateSummary,
    };
    chunk_from_range(
        file_path,
        source,
        kind,
        TextRange::new(0.into(), (source.len() as u32).into()),
        ChunkMetadata {
            symbol_path: Some(module_path.to_string()),
            symbol_name: file_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToString::to_string),
            parent_symbol_path: None,
            retrieval_markers: root_markers(root_kind),
            repository_state_hints: root_state_hints(root_kind),
            ..ChunkMetadata::default()
        },
    )
}

fn chunk_from_range(
    file_path: &Path,
    source: &str,
    kind: ChunkKind,
    range: TextRange,
    metadata: ChunkMetadata,
) -> Chunk {
    let start = u32::from(range.start());
    let end = u32::from(range.end());
    let text = metadata
        .text_override
        .clone()
        .unwrap_or_else(|| source[start as usize..end as usize].to_string());
    let id = match metadata.id_suffix.as_deref() {
        Some(suffix) => format!(
            "{}:{}:{}:{}:{}",
            file_path.display(),
            start,
            end,
            chunk_kind_label(&kind),
            suffix
        ),
        None => format!(
            "{}:{}:{}:{}",
            file_path.display(),
            start,
            end,
            chunk_kind_label(&kind)
        ),
    };

    Chunk {
        id,
        kind,
        file_path: file_path.to_path_buf(),
        span: SourceSpan {
            start_byte: start,
            end_byte: end,
        },
        symbol_path: metadata.symbol_path,
        symbol_name: metadata.symbol_name,
        owning_symbol_header: metadata.owning_symbol_header,
        docs_text: metadata.docs_text,
        signature_text: metadata.signature_text,
        parent_symbol_path: metadata.parent_symbol_path,
        retrieval_markers: metadata.retrieval_markers,
        repository_state_hints: metadata.repository_state_hints,
        text,
    }
}

fn chunk_kind_label(kind: &ChunkKind) -> &'static str {
    match kind {
        ChunkKind::CrateSummary => "crate",
        ChunkKind::ModuleSummary => "module",
        ChunkKind::Symbol => "symbol",
        ChunkKind::BodyRegion => "body",
        ChunkKind::TestFunction => "test",
        ChunkKind::ExampleFile => "example",
        ChunkKind::Doctest => "doctest",
        ChunkKind::DocumentBlock => "document",
        ChunkKind::TaskRow => "task-row",
    }
}

fn root_markers(kind: SourceRootKind) -> Vec<String> {
    match kind {
        SourceRootKind::Src => Vec::new(),
        SourceRootKind::Examples => vec!["example".to_string()],
        SourceRootKind::IntegrationTests => vec!["test".to_string()],
    }
}

fn root_state_hints(kind: SourceRootKind) -> Vec<String> {
    match kind {
        SourceRootKind::Src => Vec::new(),
        SourceRootKind::Examples => vec!["examples".to_string()],
        SourceRootKind::IntegrationTests => vec!["tests".to_string()],
    }
}

fn impl_symbol_path(source: &str, module_path: &str, range: TextRange) -> String {
    let start = u32::from(range.start()) as usize;
    let end = u32::from(range.end()) as usize;
    let snippet = &source[start..end];
    match parse_impl_target_name(snippet) {
        Some(target) => format!("{module_path}::impl::{target}"),
        None => format!("{module_path}::impl"),
    }
}

fn parse_impl_target_name(snippet: &str) -> Option<&str> {
    let impl_text = snippet.trim_start().strip_prefix("impl ")?;
    let head = impl_text
        .split(|ch: char| ch == '{' || ch == '<' || ch.is_whitespace())
        .next()?;
    if head.is_empty() { None } else { Some(head) }
}

fn item_signature(source: &str, full_range: TextRange, body_range: Option<TextRange>) -> String {
    match body_range {
        Some(body_range) => symbol_header(source, full_range, body_range),
        None => {
            let start = u32::from(full_range.start()) as usize;
            let end = u32::from(full_range.end()) as usize;
            source[start..end].trim_end().to_string()
        }
    }
}

fn symbol_header(source: &str, full_range: TextRange, body_range: TextRange) -> String {
    let start = u32::from(full_range.start()) as usize;
    let body_start = u32::from(body_range.start()) as usize;
    source[start..body_start].trim_end().to_string()
}

fn span_len(range: TextRange) -> usize {
    u32::from(range.end()) as usize - u32::from(range.start()) as usize
}

fn item_name(item: &impl HasName) -> Result<String, String> {
    item.name()
        .map(|name| name.text().to_string())
        .ok_or_else(|| "expected named item".to_string())
}

fn crate_name(root: &Path) -> Result<String, String> {
    let manifest = fs::read_to_string(root.join("Cargo.toml")).map_err(|err| err.to_string())?;
    manifest
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("name = ")
                .map(|value| value.trim_matches('"').to_string())
        })
        .ok_or_else(|| "missing crate name".to_string())
}

fn module_path_for_file(crate_name: &str, relative: &Path, root_kind: SourceRootKind) -> String {
    let mut segments = vec![crate_name.to_string()];
    match root_kind {
        SourceRootKind::Src => {}
        SourceRootKind::Examples => segments.push("examples".to_string()),
        SourceRootKind::IntegrationTests => segments.push("integration_tests".to_string()),
    }

    let mut stem_path = relative.to_path_buf();
    stem_path.set_extension("");

    let stem_name = stem_path.as_os_str();
    if stem_name != "lib" && stem_name != "main" {
        segments.extend(
            stem_path
                .components()
                .map(|component| component.as_os_str().to_string_lossy().to_string()),
        );
    }

    segments.join("::")
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }

    Ok(())
}

fn collect_document_chunks(
    root: &Path,
    crate_name: &str,
    document_sources: &DocumentSourcesConfig,
    chunks: &mut Vec<Chunk>,
) -> Result<(), String> {
    let mut files = Vec::new();
    collect_document_files(root, &mut files)?;
    files.sort();
    for file_path in files {
        let relative = file_path
            .strip_prefix(root)
            .map_err(|err| err.to_string())?
            .to_string_lossy()
            .to_string();
        let Some(rule) = classify_document_source(&relative, &document_sources.rules) else {
            continue;
        };
        let body = fs::read_to_string(&file_path).map_err(|err| err.to_string())?;
        let rank_weight_marker = format!("doc_rank_weight:{:.3}", rule.weight);
        match rule.parser {
            DocumentSourceParser::Markdown => {
                for section in chunk_markdown(&file_path, &body, rule.kind.as_str())? {
                    let heading = if section.heading_path.is_empty() {
                        "root".to_string()
                    } else {
                        section.heading_path.join("::")
                    };
                    let symbol_path = format!("{crate_name}::docs::{heading}");
                    chunks.push(Chunk {
                        id: section.id,
                        kind: ChunkKind::DocumentBlock,
                        file_path: file_path.clone(),
                        span: SourceSpan {
                            start_byte: section.start_line,
                            end_byte: section.end_line,
                        },
                        symbol_path: Some(symbol_path),
                        symbol_name: section.heading_path.last().cloned(),
                        owning_symbol_header: None,
                        docs_text: None,
                        signature_text: None,
                        parent_symbol_path: Some(format!("{crate_name}::docs")),
                        retrieval_markers: vec![
                            "document".to_string(),
                            rule.kind.as_str().to_string(),
                            rank_weight_marker.clone(),
                        ],
                        repository_state_hints: vec!["docs".to_string()],
                        text: section.text,
                    });
                }
            }
            DocumentSourceParser::Csv => {
                for row in chunk_csv_rows(&file_path, &body, rule.kind.as_str())? {
                    let symbol_path = format!("{crate_name}::docs::tasks::row_{}", row.row_number);
                    chunks.push(Chunk {
                        id: row.id,
                        kind: ChunkKind::TaskRow,
                        file_path: file_path.clone(),
                        span: SourceSpan {
                            start_byte: row.row_number,
                            end_byte: row.row_number,
                        },
                        symbol_path: Some(symbol_path),
                        symbol_name: Some(format!("row_{}", row.row_number)),
                        owning_symbol_header: None,
                        docs_text: None,
                        signature_text: None,
                        parent_symbol_path: Some(format!("{crate_name}::docs::tasks")),
                        retrieval_markers: vec![
                            "document".to_string(),
                            rule.kind.as_str().to_string(),
                            rank_weight_marker.clone(),
                        ],
                        repository_state_hints: vec!["docs".to_string(), "tasks".to_string()],
                        text: row.text,
                    });
                }
            }
        }
    }
    Ok(())
}

fn classify_document_source(
    relative_path: &str,
    rules: &[DocumentSourceRule],
) -> Option<DocumentSourceRule> {
    rules
        .iter()
        .find(|rule| path_matches_glob(relative_path, &rule.path_glob))
        .cloned()
}

fn path_matches_glob(path: &str, glob: &str) -> bool {
    if let Some(prefix) = glob.strip_suffix("/**") {
        return path.starts_with(prefix);
    }
    path == glob
}

fn collect_document_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| matches!(name, ".git" | "target"))
            {
                continue;
            }
            collect_document_files(&path, files)?;
            continue;
        }
        let is_doc = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("csv"));
        let is_changelog = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "CHANGELOG.md");
        if is_doc || is_changelog {
            files.push(path);
        }
    }
    Ok(())
}
