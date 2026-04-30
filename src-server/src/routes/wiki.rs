use axum::{
    extract::Query,
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RelatedPagesQuery {
    project_path: String,
    source_name: String,
}

/// Find wiki pages related to a source file.
/// This mirrors the Rust find_related_wiki_pages logic from src-tauri/src/commands/fs.rs
pub async fn find_related_pages(
    Query(query): Query<RelatedPagesQuery>,
) -> Result<Json<Vec<String>>, String> {
    let source_slug = query.source_name
        .split('.')
        .next()
        .unwrap_or(&query.source_name);

    let wiki_dir = std::path::Path::new(&query.project_path).join("wiki");
    if !wiki_dir.exists() {
        return Ok(Json(Vec::new()));
    }

    let mut related = Vec::new();

    // Walk through wiki directory looking for .md files
    fn walk_dir(dir: &std::path::Path, source_slug: &str, source_name: &str, related: &mut Vec<String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk_dir(&path, source_slug, source_name, related);
                } else if path.extension().map_or(false, |ext| ext == "md") {
                    let file_name = path.file_name()
                        .map(|n| n.to_string_lossy())
                        .map(|cow| cow.into_owned())
                        .unwrap_or_default();

                    // Strategy 1: Direct match on slug
                    if file_name.starts_with(source_slug) {
                        related.push(path.to_string_lossy().to_string());
                        continue;
                    }

                    // Strategy 2: Check sources list in frontmatter
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if content.contains(source_name) {
                            related.push(path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    walk_dir(&wiki_dir, source_slug, &query.source_name, &mut related);

    Ok(Json(related))
}