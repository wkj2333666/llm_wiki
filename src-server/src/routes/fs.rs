use axum::{
    extract::Query,
    Json,
    response::Response,
    body::Body,
};
use serde::{Deserialize, Serialize};

use crate::types::FileNode;

#[derive(Deserialize)]
pub struct ReadFileQuery {
    path: String,
}

#[derive(Serialize)]
pub struct ReadFileResponse {
    content: String,
}

/// Read file content (with PDF/Office extraction)
pub async fn read_file(
    Query(query): Query<ReadFileQuery>,
) -> Result<Json<ReadFileResponse>, String> {
    // TODO: implement using services::fs::read_file
    let content = std::fs::read_to_string(&query.path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    Ok(Json(ReadFileResponse { content }))
}

#[derive(Deserialize)]
pub struct WriteFileBody {
    path: String,
    content: String,
}

/// Write file content
pub async fn write_file(
    Json(body): Json<WriteFileBody>,
) -> Result<(), String> {
    // TODO: implement using services::fs::write_file
    std::fs::write(&body.path, &body.content)
        .map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}

#[derive(Deserialize)]
pub struct ListDirectoryQuery {
    path: String,
}

/// List directory contents
pub async fn list_directory(
    Query(query): Query<ListDirectoryQuery>,
) -> Result<Json<Vec<FileNode>>, String> {
    // TODO: implement using services::fs::list_directory
    let mut nodes = Vec::new();
    let entries = std::fs::read_dir(&query.path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = path.is_dir();

        nodes.push(FileNode {
            name,
            path: path.to_string_lossy().to_string(),
            is_dir,
            children: None,
        });
    }

    Ok(Json(nodes))
}

#[derive(Deserialize)]
pub struct CopyFileBody {
    source: String,
    destination: String,
}

/// Copy file
pub async fn copy_file(
    Json(body): Json<CopyFileBody>,
) -> Result<(), String> {
    // TODO: implement using services::fs::copy_file
    std::fs::copy(&body.source, &body.destination)
        .map_err(|e| format!("Failed to copy file: {}", e))?;
    Ok(())
}

#[derive(Deserialize)]
pub struct CopyDirectoryBody {
    source: String,
    destination: String,
}

/// Copy directory recursively
pub async fn copy_directory(
    Json(body): Json<CopyDirectoryBody>,
) -> Result<Json<Vec<String>>, String> {
    // TODO: implement using services::fs::copy_directory
    let mut copied = Vec::new();
    // Simplified implementation - just copy the directory
    fs_extra::copy_items(&[&body.source], &body.destination, &fs_extra::dir::CopyOptions::new())
        .map_err(|e| format!("Failed to copy directory: {}", e))?;
    copied.push(body.destination);
    Ok(Json(copied))
}

#[derive(Deserialize)]
pub struct PreprocessFileBody {
    path: String,
}

/// Preprocess file and cache extracted text
pub async fn preprocess_file(
    Json(_body): Json<PreprocessFileBody>,
) -> Result<Json<String>, String> {
    // TODO: implement using services::fs::preprocess_file
    Ok(Json("no preprocessing needed".to_string()))
}

#[derive(Deserialize)]
pub struct DeleteFileBody {
    path: String,
}

/// Delete file or directory
pub async fn delete_file(
    Json(body): Json<DeleteFileBody>,
) -> Result<(), String> {
    // TODO: implement using services::fs::delete_file
    let path = std::path::Path::new(&body.path);
    if path.is_dir() {
        std::fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to remove directory: {}", e))?;
    } else {
        std::fs::remove_file(path)
            .map_err(|e| format!("Failed to remove file: {}", e))?;
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct CreateDirectoryBody {
    path: String,
}

/// Create directory
pub async fn create_directory(
    Json(body): Json<CreateDirectoryBody>,
) -> Result<(), String> {
    // TODO: implement using services::fs::create_directory
    std::fs::create_dir_all(&body.path)
        .map_err(|e| format!("Failed to create directory: {}", e))?;
    Ok(())
}

#[derive(Deserialize)]
pub struct FileExistsQuery {
    path: String,
}

/// Check if file exists
pub async fn file_exists(
    Query(query): Query<FileExistsQuery>,
) -> Result<Json<bool>, String> {
    // TODO: implement using services::fs::file_exists
    Ok(Json(std::path::Path::new(&query.path).exists()))
}

#[derive(Deserialize)]
pub struct ReadFileBase64Query {
    path: String,
}

#[derive(Serialize)]
pub struct FileBase64 {
    base64: String,
    mime_type: String,
}

/// Read file as base64 with mime type
pub async fn read_file_as_base64(
    Query(query): Query<ReadFileBase64Query>,
) -> Result<Json<FileBase64>, String> {
    // TODO: implement using services::fs::read_file_as_base64
    let content = std::fs::read(&query.path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    let base64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &content);

    // Guess mime type from extension
    let mime_type = mime_guess::from_path(&query.path)
        .first_or_octet_stream()
        .to_string();

    Ok(Json(FileBase64 { base64, mime_type }))
}

#[derive(Deserialize)]
pub struct ServeFileQuery {
    path: String,
}

/// Serve a file directly (for images, videos, etc.)
pub async fn serve_file(
    Query(query): Query<ServeFileQuery>,
) -> Result<Response, String> {
    let path = std::path::Path::new(&query.path);
    if !path.exists() {
        return Err("File not found".to_string());
    }

    let content = std::fs::read(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mime_type = mime_guess::from_path(&query.path)
        .first_or_octet_stream()
        .to_string();

    Ok(Response::builder()
        .header("Content-Type", mime_type)
        .header("Cache-Control", "public, max-age=3600")
        .body(Body::from(content))
        .unwrap())
}

#[derive(Deserialize)]
pub struct ExtractImagesBody {
    source_path: String,
    dest_dir: String,
    rel_to: String,
    #[serde(rename = "isPdf")]
    is_pdf: bool,
}

#[derive(Serialize)]
pub struct SavedImage {
    index: i32,
    mime_type: String,
    page: Option<i32>,
    width: i32,
    height: i32,
    #[serde(rename = "relPath")]
    rel_path: String,
    #[serde(rename = "absPath")]
    abs_path: String,
    sha256: String,
}

/// Extract images from PDF or Office documents
pub async fn extract_images(
    Json(body): Json<ExtractImagesBody>,
) -> Result<Json<Vec<SavedImage>>, String> {
    // TODO: Implement actual image extraction using pdfium-render or calamine
    // For now, return empty array as placeholder
    // The full implementation would mirror src-tauri/src/commands/extract_images.rs
    tracing::warn!(
        "Image extraction requested for {} but not yet implemented",
        body.source_path
    );
    Ok(Json(Vec::new()))
}