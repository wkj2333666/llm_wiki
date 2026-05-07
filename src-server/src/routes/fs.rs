use axum::{
    extract::Query,
    extract::Multipart,
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
    let content = crate::services::fs::read_file(query.path).await?;
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
    crate::services::fs::write_file(body.path, body.content).await
}

#[derive(Deserialize)]
pub struct ListDirectoryQuery {
    path: String,
}

/// List directory contents
pub async fn list_directory(
    Query(query): Query<ListDirectoryQuery>,
) -> Result<Json<Vec<FileNode>>, String> {
    crate::services::fs::list_directory(query.path).await.map(Json)
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
    crate::services::fs::copy_file(body.source, body.destination).await
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
    crate::services::fs::copy_directory(body.source, body.destination).await.map(Json)
}

#[derive(Deserialize)]
pub struct PreprocessFileBody {
    path: String,
}

/// Preprocess file and cache extracted text
pub async fn preprocess_file(
    Json(body): Json<PreprocessFileBody>,
) -> Result<Json<String>, String> {
    crate::services::fs::preprocess_file(body.path).await.map(Json)
}

#[derive(Deserialize)]
pub struct DeleteFileBody {
    path: String,
}

/// Delete file or directory
pub async fn delete_file(
    Json(body): Json<DeleteFileBody>,
) -> Result<(), String> {
    crate::services::fs::delete_file(body.path).await
}

#[derive(Deserialize)]
pub struct CreateDirectoryBody {
    path: String,
}

/// Create directory
pub async fn create_directory(
    Json(body): Json<CreateDirectoryBody>,
) -> Result<(), String> {
    crate::services::fs::create_directory(body.path).await
}

#[derive(Deserialize)]
pub struct FileExistsQuery {
    path: String,
}

/// Check if file exists
pub async fn file_exists(
    Query(query): Query<FileExistsQuery>,
) -> Result<Json<bool>, String> {
    crate::services::fs::file_exists(query.path).await.map(Json)
}

#[derive(Deserialize)]
pub struct ReadFileBase64Query {
    path: String,
}

/// Read file as base64 with mime type
pub async fn read_file_as_base64(
    Query(query): Query<ReadFileBase64Query>,
) -> Result<Json<crate::services::fs::FileBase64>, String> {
    crate::services::fs::read_file_as_base64(query.path).await.map(Json)
}

#[derive(Deserialize)]
pub struct ServeFileQuery {
    path: String,
}

/// Serve a file directly (for images, videos, etc.)
pub async fn serve_file(
    Query(query): Query<ServeFileQuery>,
) -> Result<Response, String> {
    let path = query.path.clone();
    let content = tokio::task::spawn_blocking(move || {
        std::fs::read(&path)
            .map_err(|e| format!("Failed to read file '{}': {}", path, e))
    })
    .await
    .map_err(|e| format!("serve_file blocking task join error: {e}"))??;

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

/// Extract images from PDF or Office documents
pub async fn extract_images(
    Json(body): Json<ExtractImagesBody>,
) -> Result<Json<Vec<crate::services::extract_images::SavedImage>>, String> {
    if body.is_pdf {
        crate::services::extract_images::extract_and_save_pdf_images_cmd(
            body.source_path, body.dest_dir, body.rel_to,
        )
        .await
        .map(Json)
    } else {
        crate::services::extract_images::extract_and_save_office_images_cmd(
            body.source_path, body.dest_dir, body.rel_to,
        )
        .await
        .map(Json)
    }
}

#[derive(Serialize)]
pub struct UploadedFile {
    name: String,
    path: String,
    size: u64,
}

/// Upload files via multipart/form-data.
///
/// The form must include a `destination` text field (the target directory
/// on the server, e.g. project path + `/raw/sources`) and one or more
/// file fields. Each file's original filename is preserved; if a file
/// with the same name already exists in the destination, a date suffix
/// is appended (e.g. `file-20260506.pdf`).
///
/// For folder uploads, the browser sends each file with its relative
/// path via the `webkitRelativePath` field — we reconstruct the
/// directory tree under the destination.
pub async fn upload_files(
    mut multipart: Multipart,
) -> Result<Json<Vec<UploadedFile>>, String> {
    let mut destination = String::new();
    let mut uploaded = Vec::new();

    // First pass: extract the destination field
    while let Some(field) = multipart.next_field().await.map_err(|e| format!("Multipart error: {}", e))? {
        let name = field.name().unwrap_or("").to_string();

        if name == "destination" {
            destination = field.text().await.map_err(|e| format!("Failed to read destination: {}", e))?;
            continue;
        }

        // It's a file field — save it
        let file_name = field.file_name().unwrap_or("unknown").to_string();
        let data = field.bytes().await.map_err(|e| format!("Failed to read file data: {}", e))?;
        let size = data.len() as u64;

        if destination.is_empty() {
            return Err("Missing 'destination' field in multipart upload".to_string());
        }

        // Check for relative path (folder upload via webkitdirectory)
        // axum multipart doesn't expose webkitRelativePath directly,
        // so we use a custom field name "relativePath" sent alongside each file.
        // If not present, we use just the filename (flat upload).

        // Ensure destination directory exists
        let (dest_dir, base_name) = if !file_name.contains('/') && !file_name.contains('\\') {
            // Flat file — save directly to destination
            (destination.clone(), file_name.clone())
        } else {
            // Folder upload: file_name contains relative path like "myfolder/sub/file.pdf"
            // Split into parent dirs and the actual filename
            let last_sep = file_name.rfind(|c: char| c == '/' || c == '\\');
            let (parent, base) = match last_sep {
                Some(idx) if idx > 0 => (&file_name[..idx], &file_name[idx + 1..]),
                _ => ("", file_name.as_str()),
            };
            let sub_dir = if parent.is_empty() {
                destination.clone()
            } else {
                format!("{}/{}", destination, parent)
            };
            std::fs::create_dir_all(&sub_dir)
                .map_err(|e| format!("Failed to create subdirectory {}: {}", sub_dir, e))?;
            (sub_dir, base.to_string())
        };

        // Build target path with deduplication
        let target_path = dedup_path(&dest_dir, &base_name);

        // Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(&target_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        // Write file
        std::fs::write(&target_path, &data)
            .map_err(|e| format!("Failed to write file {}: {}", target_path, e))?;

        uploaded.push(UploadedFile {
            name: base_name,
            path: target_path,
            size,
        });
    }

    tracing::info!("Uploaded {} files to {}", uploaded.len(), destination);
    Ok(Json(uploaded))
}

/// If the file already exists at the destination, append a date suffix.
/// If that also exists, add a counter.
fn dedup_path(dir: &str, filename: &str) -> String {
    let base = format!("{}/{}", dir, filename);
    if !std::path::Path::new(&base).exists() {
        return base;
    }

    let ext = if filename.contains('.') {
        filename.rfind('.').map(|i| &filename[i..]).unwrap_or("")
    } else {
        ""
    };
    let name_without_ext = if ext.is_empty() {
        filename.to_string()
    } else {
        filename[..filename.len() - ext.len()].to_string()
    };
    let date = chrono::Local::now().format("%Y%m%d").to_string();

    let with_date = format!("{}/{}-{}{}", dir, name_without_ext, date, ext);
    if !std::path::Path::new(&with_date).exists() {
        return with_date;
    }

    for i in 2..100 {
        let with_counter = format!("{}/{}-{}-{}{}", dir, name_without_ext, date, i, ext);
        if !std::path::Path::new(&with_counter).exists() {
            return with_counter;
        }
    }

    format!("{}/{}-{}-{}{}", dir, name_without_ext, date, std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), ext)
}