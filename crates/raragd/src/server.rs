use std::path::{Path, PathBuf};
use std::sync::Arc;

use rarag_core::chunking::RustChunker;
use rarag_core::config::AppConfig;
use rarag_core::config_loader::load_app_config_with_source;
use rarag_core::daemon::{
    DaemonRequest, DaemonResponse, IndexResponse, QueryPayload, ReloadResponse, StatusPayload,
};
use rarag_core::embeddings::{
    DeterministicEmbeddingProvider, EmbeddingProvider, OpenAiCompatibleEmbeddings,
};
use rarag_core::indexing::{ChunkIndexer, LanceDbPointStore, TantivyChunkStore};
use rarag_core::metadata::SnapshotStore;
use rarag_core::retrieval::{QueryMode, RepositoryRetriever};
use rarag_core::unix_socket::{prepare_socket_path, remove_socket_if_present};
use tokio::net::UnixListener;
#[cfg(unix)]
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::Mutex;

use crate::config::ServeConfig;
use crate::transport::{error_response, read_request, write_response};

enum DaemonEmbeddingProvider {
    OpenAi(OpenAiCompatibleEmbeddings),
    Deterministic(DeterministicEmbeddingProvider),
}

impl EmbeddingProvider for DaemonEmbeddingProvider {
    fn embed_texts(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, String> {
        match self {
            Self::OpenAi(provider) => provider.embed_texts(inputs),
            Self::Deterministic(provider) => provider.embed_texts(inputs),
        }
    }
}

pub struct DaemonState {
    metadata: SnapshotStore,
    tantivy: TantivyChunkStore,
    lancedb: LanceDbPointStore,
    provider: DaemonEmbeddingProvider,
    active_config: AppConfig,
    config_source: Option<PathBuf>,
    config_generation: u64,
}

impl DaemonState {
    async fn open(
        config: AppConfig,
        config_source: Option<PathBuf>,
        serve: ServeConfig,
    ) -> Result<Self, String> {
        let metadata_path = local_database_path(&config.turso.database_url)?;
        if let Some(parent) = metadata_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        let metadata = SnapshotStore::open_local(&metadata_path.display().to_string()).await?;
        let tantivy = TantivyChunkStore::open(Path::new(&config.tantivy.index_root))?;
        let lancedb = if serve.memory_vector_store {
            LanceDbPointStore::new_in_memory_with_metric(
                "memory://daemon-test",
                &config.lancedb.table,
                config.embeddings.dimensions,
                config.lancedb.distance_metric,
            )
        } else {
            LanceDbPointStore::new_with_metric(
                &config.lancedb.db_root,
                &config.lancedb.table,
                config.embeddings.dimensions,
                config.lancedb.distance_metric,
            )?
        };
        let provider = if serve.deterministic_embeddings {
            DaemonEmbeddingProvider::Deterministic(DeterministicEmbeddingProvider::new(
                config.embeddings.dimensions,
            )?)
        } else {
            DaemonEmbeddingProvider::OpenAi(OpenAiCompatibleEmbeddings::from_config(
                &config.embeddings,
            )?)
        };

        Ok(Self {
            metadata,
            tantivy,
            lancedb,
            provider,
            active_config: config,
            config_source,
            config_generation: 0,
        })
    }

    async fn handle_request(&mut self, request: DaemonRequest) -> DaemonResponse {
        match request {
            DaemonRequest::Status {
                snapshot_id,
                worktree_root,
            } => match self.resolve_snapshot(snapshot_id, worktree_root).await {
                Ok(snapshot_id) => DaemonResponse::Status(StatusPayload {
                    resolved_snapshot_id: snapshot_id,
                    warnings: Vec::new(),
                }),
                Err(err) => error_response(err),
            },
            DaemonRequest::IndexWorkspace {
                snapshot,
                workspace_root,
                max_body_bytes,
            } => match self
                .index_workspace(snapshot, Path::new(&workspace_root), max_body_bytes)
                .await
            {
                Ok(response) => DaemonResponse::Indexed(response),
                Err(err) => error_response(err),
            },
            DaemonRequest::Query(payload) => self.query(payload).await,
            DaemonRequest::BlastRadius(mut payload) => {
                payload.query_mode = QueryMode::BlastRadius;
                self.query(payload).await
            }
            DaemonRequest::ReloadConfig => match self.reload_config().await {
                Ok(response) => DaemonResponse::Reloaded(response),
                Err(err) => error_response(err),
            },
            DaemonRequest::Shutdown => DaemonResponse::Ack,
        }
    }

    async fn reload_config(&mut self) -> Result<ReloadResponse, String> {
        let loaded = load_app_config_with_source(self.config_source.as_deref())?;
        self.active_config.retrieval = loaded.config.retrieval;
        self.active_config.observability = loaded.config.observability;
        self.active_config.document_sources = loaded.config.document_sources;
        self.active_config.history = loaded.config.history;
        self.config_source = loaded.source_path;
        self.config_generation += 1;
        Ok(ReloadResponse {
            generation: self.config_generation,
            source_path: self
                .config_source
                .as_ref()
                .map(|path| path.display().to_string()),
        })
    }

    async fn index_workspace(
        &mut self,
        snapshot: rarag_core::snapshot::SnapshotKey,
        workspace_root: &Path,
        max_body_bytes: usize,
    ) -> Result<IndexResponse, String> {
        let snapshot = self.metadata.create_or_get_snapshot(snapshot).await?;
        let chunks = RustChunker::new_with_document_sources(
            max_body_bytes,
            self.active_config.document_sources.clone(),
        )
        .chunk_workspace(workspace_root)?;
        let indexer =
            ChunkIndexer::new(&self.metadata, &self.tantivy, &self.lancedb, &self.provider);
        let counts = indexer.reindex_snapshot(&snapshot.id, &chunks).await?;
        Ok(IndexResponse {
            snapshot_id: snapshot.id,
            chunk_count: counts.metadata_rows,
            warnings: Vec::new(),
        })
    }

    async fn query(&mut self, payload: QueryPayload) -> DaemonResponse {
        if let Err(err) = payload.validate_locator() {
            return error_response(err);
        }

        let snapshot_id = match self
            .resolve_snapshot(payload.snapshot_id.clone(), payload.worktree_root.clone())
            .await
        {
            Ok(Some(snapshot_id)) => snapshot_id,
            Ok(None) => return error_response("no snapshot found for request"),
            Err(err) => return error_response(err),
        };

        let request = payload.into_retrieval_request(snapshot_id);
        let retriever = RepositoryRetriever::new_with_full_settings(
            &self.metadata,
            &self.tantivy,
            &self.lancedb,
            &self.provider,
            &self.active_config.retrieval,
            &self.active_config.observability,
            &self.active_config.document_sources,
        );
        match retriever.retrieve(request).await {
            Ok(response) => DaemonResponse::Query(response),
            Err(err) => error_response(err),
        }
    }

    async fn resolve_snapshot(
        &self,
        snapshot_id: Option<String>,
        worktree_root: Option<String>,
    ) -> Result<Option<String>, String> {
        if let Some(snapshot_id) = snapshot_id {
            return Ok(Some(snapshot_id));
        }
        if let Some(worktree_root) = worktree_root {
            let snapshot = self
                .metadata
                .resolve_snapshot_for_worktree_root(&worktree_root)
                .await?;
            return Ok(snapshot.map(|snapshot| snapshot.id));
        }

        Err("requests require snapshot_id or worktree_root".to_string())
    }
}

pub async fn serve(
    config: AppConfig,
    config_source: Option<PathBuf>,
    serve: ServeConfig,
) -> Result<(), String> {
    let socket_path = serve
        .socket_path
        .clone()
        .unwrap_or_else(|| PathBuf::from(config.daemon_socket_path()));
    prepare_socket_path(&socket_path)?;

    let listener = UnixListener::bind(&socket_path).map_err(|err| err.to_string())?;
    let state = Arc::new(Mutex::new(
        DaemonState::open(config, config_source, serve.clone()).await?,
    ));

    #[cfg(unix)]
    let mut sighup = signal(SignalKind::hangup()).map_err(|err| err.to_string())?;

    #[cfg(unix)]
    loop {
        tokio::select! {
            maybe_signal = sighup.recv() => {
                if maybe_signal.is_some() {
                    let result = {
                        let mut state = state.lock().await;
                        state.reload_config().await
                    };
                    if let Err(err) = result {
                        eprintln!("config reload failed: {err}");
                    }
                }
            }
            accepted = listener.accept() => {
                let (mut stream, _) = accepted.map_err(|err| err.to_string())?;
                let request = match read_request(&mut stream).await {
                    Ok(request) => request,
                    Err(err) => {
                        write_response(&mut stream, &error_response(err)).await?;
                        continue;
                    }
                };
                let response = {
                    let mut state = state.lock().await;
                    state.handle_request(request).await
                };
                let shutdown = matches!(response, DaemonResponse::Ack);
                write_response(&mut stream, &response).await?;
                if shutdown {
                    break;
                }
            }
        }
    }

    #[cfg(not(unix))]
    loop {
        let (mut stream, _) = listener.accept().await.map_err(|err| err.to_string())?;
        let request = match read_request(&mut stream).await {
            Ok(request) => request,
            Err(err) => {
                write_response(&mut stream, &error_response(err)).await?;
                continue;
            }
        };
        let response = {
            let mut state = state.lock().await;
            state.handle_request(request).await
        };
        let shutdown = matches!(response, DaemonResponse::Ack);
        write_response(&mut stream, &response).await?;
        if shutdown {
            break;
        }
    }

    remove_socket_if_present(&socket_path)?;
    Ok(())
}

fn local_database_path(database_url: &str) -> Result<PathBuf, String> {
    if let Some(path) = database_url.strip_prefix("file:") {
        Ok(PathBuf::from(path))
    } else {
        Err(format!("unsupported local database url: {database_url}"))
    }
}
