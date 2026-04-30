use axum::Json;
use serde::{Deserialize, Serialize};

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
    Json(_body): Json<UpsertChunksBody>,
) -> Result<Json<UpsertResponse>, String> {
    // TODO: implement using services::vector::upsert_chunks
    Ok(Json(UpsertResponse { success: true }))
}

#[derive(Deserialize)]
pub struct SearchChunksBody {
    project_path: String,
    query_embedding: Vec<f32>,
    limit: Option<u32>,
}

#[derive(Serialize)]
pub struct ChunkSearchResult {
    chunk_id: String,
    page_id: String,
    chunk_index: u32,
    chunk_text: String,
    heading_path: String,
    score: f32,
}

/// Search chunks by embedding
pub async fn search_chunks(
    Json(_body): Json<SearchChunksBody>,
) -> Result<Json<Vec<ChunkSearchResult>>, String> {
    // TODO: implement using services::vector::search_chunks
    Ok(Json(Vec::new()))
}

#[derive(Deserialize)]
pub struct DeletePageBody {
    project_path: String,
    page_id: String,
}

/// Delete page embeddings
pub async fn delete_page(
    Json(_body): Json<DeletePageBody>,
) -> Result<(), String> {
    // TODO: implement using services::vector::delete_page
    Ok(())
}

#[derive(Deserialize)]
pub struct CountChunksQuery {
    project_path: String,
}

#[derive(Serialize)]
pub struct CountResponse {
    count: u64,
}

/// Count chunks in vector store
pub async fn count_chunks(
    Json(_query): Json<CountChunksQuery>,
) -> Result<Json<CountResponse>, String> {
    // TODO: implement using services::vector::count_chunks
    Ok(Json(CountResponse { count: 0 }))
}