use std::fs;
use std::path::{Path, PathBuf};

use ra_ap_syntax::{
    AstNode, Edition, SourceFile, TextRange,
    ast::{self, HasAttrs, HasModuleItem, HasName},
};

use crate::chunking::types::{Chunk, ChunkKind, SourceSpan};

#[derive(Debug, Clone, Copy)]
pub struct RustChunker {
    max_body_bytes: usize,
}

impl RustChunker {
    pub fn new(max_body_bytes: usize) -> Self {
        Self { max_body_bytes }
    }

    pub fn chunk_workspace(&self, root: &Path) -> Result<Vec<Chunk>, String> {
        let crate_name = crate_name(root)?;
        let mut rust_files = Vec::new();
        collect_rust_files(&root.join("src"), &mut rust_files)?;
        rust_files.sort();

        let mut chunks = Vec::new();
        for file_path in rust_files {
            let relative = file_path
                .strip_prefix(root.join("src"))
                .map_err(|err| err.to_string())?;
            let module_path = module_path_for_file(&crate_name, relative);
            let source = fs::read_to_string(&file_path).map_err(|err| err.to_string())?;
            let parse = SourceFile::parse(&source, Edition::CURRENT);
            let source_file = parse.tree();
            let items: Vec<_> = source_file.items().collect();

            chunks.push(file_chunk(&file_path, &source, &module_path));
            collect_items(
                &source,
                &file_path,
                &module_path,
                items,
                self.max_body_bytes,
                true,
                &mut chunks,
            )?;
        }

        Ok(chunks)
    }
}

fn collect_items(
    source: &str,
    file_path: &Path,
    module_path: &str,
    items: Vec<ast::Item>,
    max_body_bytes: usize,
    collect_tests: bool,
    chunks: &mut Vec<Chunk>,
) -> Result<(), String> {
    for item in &items {
        match item {
            ast::Item::Fn(function) => {
                let name = item_name(function)?;
                let symbol_path = format!("{module_path}::{name}");
                let syntax = function.syntax();
                chunks.push(chunk_from_range(
                    file_path,
                    source,
                    ChunkKind::Symbol,
                    syntax.text_range(),
                    Some(symbol_path.clone()),
                    None,
                ));

                if let Some(body) = function.body() {
                    let body_range = body.syntax().text_range();
                    if span_len(body_range) > max_body_bytes {
                        chunks.push(chunk_from_range(
                            file_path,
                            source,
                            ChunkKind::BodyRegion,
                            body_range,
                            Some(symbol_path.clone()),
                            Some(symbol_header(source, syntax.text_range(), body_range)),
                        ));
                    }
                }
            }
            ast::Item::Struct(strukt) => {
                let name = item_name(strukt)?;
                chunks.push(chunk_from_range(
                    file_path,
                    source,
                    ChunkKind::Symbol,
                    strukt.syntax().text_range(),
                    Some(format!("{module_path}::{name}")),
                    None,
                ));
            }
            ast::Item::Module(module) => {
                let name = item_name(module)?;
                let nested_path = format!("{module_path}::{name}");
                chunks.push(chunk_from_range(
                    file_path,
                    source,
                    ChunkKind::ModuleSummary,
                    module.syntax().text_range(),
                    Some(nested_path.clone()),
                    None,
                ));

                if let Some(item_list) = module.item_list() {
                    collect_items(
                        source,
                        file_path,
                        &nested_path,
                        item_list.items().collect(),
                        max_body_bytes,
                        false,
                        chunks,
                    )?;
                }
            }
            ast::Item::Enum(enm) => {
                let name = item_name(enm)?;
                chunks.push(chunk_from_range(
                    file_path,
                    source,
                    ChunkKind::Symbol,
                    enm.syntax().text_range(),
                    Some(format!("{module_path}::{name}")),
                    None,
                ));
            }
            ast::Item::Trait(trait_item) => {
                let name = item_name(trait_item)?;
                chunks.push(chunk_from_range(
                    file_path,
                    source,
                    ChunkKind::Symbol,
                    trait_item.syntax().text_range(),
                    Some(format!("{module_path}::{name}")),
                    None,
                ));
            }
            ast::Item::Impl(imp) => {
                chunks.push(chunk_from_range(
                    file_path,
                    source,
                    ChunkKind::Symbol,
                    imp.syntax().text_range(),
                    Some(format!("{module_path}::impl")),
                    None,
                ));
            }
            _ => {}
        }
    }

    if collect_tests {
        collect_test_functions(source, file_path, module_path, items, chunks)?;
    }
    Ok(())
}

fn collect_test_functions(
    source: &str,
    file_path: &Path,
    module_path: &str,
    items: Vec<ast::Item>,
    chunks: &mut Vec<Chunk>,
) -> Result<(), String> {
    for item in items {
        match item {
            ast::Item::Fn(function) if function.has_atom_attr("test") => {
                let name = item_name(&function)?;
                chunks.push(chunk_from_range(
                    file_path,
                    source,
                    ChunkKind::TestFunction,
                    function.syntax().text_range(),
                    Some(format!("{module_path}::{name}")),
                    None,
                ));
            }
            ast::Item::Module(module) => {
                let name = item_name(&module)?;
                if let Some(item_list) = module.item_list() {
                    collect_test_functions(
                        source,
                        file_path,
                        &format!("{module_path}::{name}"),
                        item_list.items().collect(),
                        chunks,
                    )?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn file_chunk(file_path: &Path, source: &str, module_path: &str) -> Chunk {
    chunk_from_range(
        file_path,
        source,
        ChunkKind::CrateSummary,
        TextRange::new(0.into(), (source.len() as u32).into()),
        Some(module_path.to_string()),
        None,
    )
}

fn chunk_from_range(
    file_path: &Path,
    source: &str,
    kind: ChunkKind,
    range: TextRange,
    symbol_path: Option<String>,
    owning_symbol_header: Option<String>,
) -> Chunk {
    let start = u32::from(range.start());
    let end = u32::from(range.end());
    let text = source[start as usize..end as usize].to_string();
    let id = format!(
        "{}:{}:{}:{}",
        file_path.display(),
        start,
        end,
        chunk_kind_label(&kind)
    );

    Chunk {
        id,
        kind,
        file_path: file_path.to_path_buf(),
        span: SourceSpan {
            start_byte: start,
            end_byte: end,
        },
        symbol_path,
        owning_symbol_header,
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

fn module_path_for_file(crate_name: &str, relative: &Path) -> String {
    let mut segments = vec![crate_name.to_string()];
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
