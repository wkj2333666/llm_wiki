use axum::Json;
use serde::{Deserialize, Serialize};

use crate::services::vector::{self, ChunkUpsertInput, ChunkSearchResult};

#[derive(Deserialize)]
pub struct UpsertChunksBody {
    project_path: String,
    page_id: String,
    chunks: Vec<ChunkInput>,
}

#[derive(Deserialize)]
pub struct ChunkInput {
    chunk_index: u32,
    chunk_text: String,
    heading_path: String,
    embedding: Vec<f32>,
}

#[derive(Serialize)]
pub struct UpsertResponse {
    success: bool,
}

/// Upsert page chunks with embeddings
pub async fn upsert_chunks(
    Json(body): Json<UpsertChunksBody>,
) -> Result<Json<UpsertResponse>, String> {
    // Convert to service input type
    let chunks: Vec<ChunkUpsertInput> = body.chunks.into_iter().map(|c| ChunkUpsertInput {
        chunk_index: c.chunk_index,
        chunk_text: c.chunk_text,
        heading_path: c.heading_path,
        embedding: c.embedding,
    }).collect();

    vector::upsert_chunks(&body.project_path, &body.page_id, &chunks).await?;

    Ok(Json(UpsertResponse { success: true }))
}

#[derive(Deserialize)]
pub struct SearchChunksBody {
    project_path: String,
    query_embedding: Vec<f32>,
    limit: Option<u32>,
}

/// Search chunks by embedding
pub async fn search_chunks(
    Json(body): Json<SearchChunksBody>,
) -> Result<Json<Vec<ChunkSearchResult>>, String> {
    let limit = body.limit.unwrap_or(10) as usize;
    let results = vector::search_chunks(&body.project_path, body.query_embedding, limit).await?;
    Ok(Json(results))
}

#[derive(Deserialize)]
pub struct DeletePageBody {
    project_path: String,
    page_id: String,
}

/// Delete page embeddings
pub async fn delete_page(
    Json(body): Json<DeletePageBody>,
) -> Result<(), String> {
    vector::delete_page(&body.project_path, &body.page_id).await?;
    Ok(())
}

#[derive(Deserialize)]
pub struct CountChunksBody {
    project_path: String,
}

#[derive(Serialize)]
pub struct CountResponse {
    count: u64,
}

/// Count chunks in vector store
pub async fn count_chunks(
    Json(body): Json<CountChunksBody>,
) -> Result<Json<CountResponse>, String> {
    let count = vector::count_chunks(&body.project_path).await?;
    Ok(Json(CountResponse { count: count as u64 }))
}