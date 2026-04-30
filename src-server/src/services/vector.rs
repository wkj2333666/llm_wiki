//! Vector store service using LanceDB
//! Ported from src-tauri/src/commands/vectorstore.rs

use lancedb::connect;
use lancedb::query::{ExecutableQuery, QueryBase};
use arrow_array::{Float32Array, RecordBatch, StringArray, FixedSizeListArray, ArrayRef, UInt32Array};
use arrow_schema::{DataType, Field, Schema};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// v2 per-chunk search result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChunkSearchResult {
    pub chunk_id: String,
    pub page_id: String,
    pub chunk_index: u32,
    pub chunk_text: String,
    pub heading_path: String,
    pub score: f32,
}

/// Input row for upsert
#[derive(Debug, Deserialize)]
pub struct ChunkUpsertInput {
    pub chunk_index: u32,
    pub chunk_text: String,
    pub heading_path: String,
    pub embedding: Vec<f32>,
}

const TABLE_V2: &str = "wiki_chunks_v2";

fn db_path(project_path: &str) -> String {
    format!("{}/.llm-wiki/lancedb", project_path.replace('\\', "/"))
}

fn validate_page_id(page_id: &str) -> Result<(), String> {
    if page_id.is_empty() || page_id.len() > 256 {
        return Err("Invalid page_id: empty or too long".to_string());
    }
    if !page_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
        return Err(format!("Invalid page_id: contains disallowed characters: {}", page_id));
    }
    Ok(())
}

fn make_schema_v2(dim: i32) -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("chunk_id", DataType::Utf8, false),
        Field::new("page_id", DataType::Utf8, false),
        Field::new("chunk_index", DataType::UInt32, false),
        Field::new("chunk_text", DataType::Utf8, false),
        Field::new("heading_path", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                dim,
            ),
            false,
        ),
    ]))
}

fn make_batch_v2(
    schema: Arc<Schema>,
    page_id: &str,
    chunks: &[ChunkUpsertInput],
    dim: i32,
) -> Result<RecordBatch, String> {
    let mut chunk_ids: Vec<String> = Vec::with_capacity(chunks.len());
    let mut page_ids: Vec<String> = Vec::with_capacity(chunks.len());
    let mut indexes: Vec<u32> = Vec::with_capacity(chunks.len());
    let mut texts: Vec<String> = Vec::with_capacity(chunks.len());
    let mut heading_paths: Vec<String> = Vec::with_capacity(chunks.len());
    let mut flat_vectors: Vec<f32> = Vec::with_capacity(chunks.len() * dim as usize);

    for c in chunks {
        if c.embedding.len() as i32 != dim {
            return Err(format!(
                "Chunk #{} has embedding dim {} but batch dim is {}",
                c.chunk_index,
                c.embedding.len(),
                dim
            ));
        }
        chunk_ids.push(format!("{}#{}", page_id, c.chunk_index));
        page_ids.push(page_id.to_string());
        indexes.push(c.chunk_index);
        texts.push(c.chunk_text.clone());
        heading_paths.push(c.heading_path.clone());
        flat_vectors.extend_from_slice(&c.embedding);
    }

    let chunk_ids_arr: ArrayRef = Arc::new(StringArray::from(chunk_ids));
    let page_ids_arr: ArrayRef = Arc::new(StringArray::from(page_ids));
    let indexes_arr: ArrayRef = Arc::new(UInt32Array::from(indexes));
    let texts_arr: ArrayRef = Arc::new(StringArray::from(texts));
    let heading_paths_arr: ArrayRef = Arc::new(StringArray::from(heading_paths));

    let values = Float32Array::from(flat_vectors);
    let vector_arr: ArrayRef = Arc::new(FixedSizeListArray::new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        dim,
        Arc::new(values),
        None,
    ));

    RecordBatch::try_new(
        schema,
        vec![chunk_ids_arr, page_ids_arr, indexes_arr, texts_arr, heading_paths_arr, vector_arr],
    ).map_err(|e| format!("Batch error: {e}"))
}

/// Upsert a batch of chunks for a single page
pub async fn upsert_chunks(
    project_path: &str,
    page_id: &str,
    chunks: &[ChunkUpsertInput],
) -> Result<(), String> {
    validate_page_id(page_id)?;

    if chunks.is_empty() {
        return Ok(());
    }

    let dim = chunks[0].embedding.len() as i32;
    if dim == 0 {
        return Err("Chunk #0 has empty embedding".to_string());
    }

    let db = connect(&db_path(project_path))
        .execute()
        .await
        .map_err(|e| format!("DB connect error: {e}"))?;

    let schema = make_schema_v2(dim);
    let batch = make_batch_v2(schema.clone(), page_id, chunks, dim)?;
    let data = vec![batch];

    let tables = db.table_names()
        .execute()
        .await
        .map_err(|e| format!("List tables error: {e}"))?;

    if tables.contains(&TABLE_V2.to_string()) {
        let table = db.open_table(TABLE_V2)
            .execute()
            .await
            .map_err(|e| format!("Open table error: {e}"))?;

        if let Err(e) = table.delete(&format!("page_id = '{}'", page_id)).await {
            tracing::warn!("Delete before upsert failed for page '{}': {}", page_id, e);
        }

        table.add(data)
            .execute()
            .await
            .map_err(|e| format!("Add error: {e}"))?;
    } else {
        db.create_table(TABLE_V2, data)
            .execute()
            .await
            .map_err(|e| format!("Create table error: {e}"))?;
    }

    Ok(())
}

/// Search chunks by embedding vector
pub async fn search_chunks(
    project_path: &str,
    query_embedding: Vec<f32>,
    limit: usize,
) -> Result<Vec<ChunkSearchResult>, String> {
    let db = connect(&db_path(project_path))
        .execute()
        .await
        .map_err(|e| format!("DB connect error: {e}"))?;

    let tables = db.table_names()
        .execute()
        .await
        .map_err(|e| format!("List tables error: {e}"))?;

    if !tables.contains(&TABLE_V2.to_string()) {
        return Ok(vec![]);
    }

    let table = db.open_table(TABLE_V2)
        .execute()
        .await
        .map_err(|e| format!("Open table error: {e}"))?;

    let results_stream = table
        .vector_search(query_embedding)
        .map_err(|e| format!("Search error: {e}"))?
        .limit(limit)
        .execute()
        .await
        .map_err(|e| format!("Execute search error: {e}"))?;

    use futures::TryStreamExt;
    let batches: Vec<RecordBatch> = results_stream
        .try_collect()
        .await
        .map_err(|e| format!("Collect error: {e}"))?;

    let mut out: Vec<ChunkSearchResult> = Vec::new();
    for batch in &batches {
        let chunk_ids = batch.column_by_name("chunk_id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or("Missing chunk_id column")?;
        let page_ids = batch.column_by_name("page_id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or("Missing page_id column")?;
        let chunk_indexes = batch.column_by_name("chunk_index")
            .and_then(|c| c.as_any().downcast_ref::<UInt32Array>())
            .ok_or("Missing chunk_index column")?;
        let chunk_texts = batch.column_by_name("chunk_text")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or("Missing chunk_text column")?;
        let heading_paths = batch.column_by_name("heading_path")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or("Missing heading_path column")?;
        let distances = batch.column_by_name("_distance")
            .and_then(|c| c.as_any().downcast_ref::<Float32Array>())
            .ok_or("Missing _distance column")?;

        for i in 0..batch.num_rows() {
            let distance = distances.value(i);
            out.push(ChunkSearchResult {
                chunk_id: chunk_ids.value(i).to_string(),
                page_id: page_ids.value(i).to_string(),
                chunk_index: chunk_indexes.value(i),
                chunk_text: chunk_texts.value(i).to_string(),
                heading_path: heading_paths.value(i).to_string(),
                score: 1.0 / (1.0 + distance),
            });
        }
    }

    Ok(out)
}

/// Delete all chunks for a page
pub async fn delete_page(project_path: &str, page_id: &str) -> Result<(), String> {
    validate_page_id(page_id)?;

    let db = connect(&db_path(project_path))
        .execute()
        .await
        .map_err(|e| format!("DB connect error: {e}"))?;

    let tables = db.table_names()
        .execute()
        .await
        .map_err(|e| format!("List tables error: {e}"))?;

    if !tables.contains(&TABLE_V2.to_string()) {
        return Ok(());
    }

    let table = db.open_table(TABLE_V2)
        .execute()
        .await
        .map_err(|e| format!("Open table error: {e}"))?;

    table.delete(&format!("page_id = '{}'", page_id))
        .await
        .map_err(|e| format!("Delete error: {e}"))?;

    Ok(())
}

/// Count total chunks in the vector store
pub async fn count_chunks(project_path: &str) -> Result<usize, String> {
    let db = connect(&db_path(project_path))
        .execute()
        .await
        .map_err(|e| format!("DB connect error: {e}"))?;

    let tables = db.table_names()
        .execute()
        .await
        .map_err(|e| format!("List tables error: {e}"))?;

    if !tables.contains(&TABLE_V2.to_string()) {
        return Ok(0);
    }

    let table = db.open_table(TABLE_V2)
        .execute()
        .await
        .map_err(|e| format!("Open table error: {e}"))?;

    let count = table.count_rows(None)
        .await
        .map_err(|e| format!("Count error: {e}"))?;

    Ok(count)
}