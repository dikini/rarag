use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkKind {
    CrateSummary,
    ModuleSummary,
    Symbol,
    BodyRegion,
    TestFunction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub start_byte: u32,
    pub end_byte: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub id: String,
    pub kind: ChunkKind,
    pub file_path: PathBuf,
    pub span: SourceSpan,
    pub symbol_path: Option<String>,
    pub owning_symbol_header: Option<String>,
    pub text: String,
}
