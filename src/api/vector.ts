/**
 * Vector store API - replaces Tauri vector commands.
 */

import { apiGet, apiPost } from './client';

interface ChunkInput {
  chunk_index: number;
  chunk_text: string;
  heading_path: string;
  embedding: number[];
}

export async function upsertChunks(
  projectPath: string,
  pageId: string,
  chunks: ChunkInput[]
): Promise<void> {
  await apiPost('/vector/upsert', { project_path: projectPath, page_id: pageId, chunks });
}

interface ChunkSearchResult {
  chunk_id: string;
  page_id: string;
  chunk_index: number;
  chunk_text: string;
  heading_path: string;
  score: number;
}

export async function searchChunks(
  projectPath: string,
  queryEmbedding: number[],
  limit?: number
): Promise<ChunkSearchResult[]> {
  return apiPost<ChunkSearchResult[]>('/vector/search', {
    project_path: projectPath,
    query_embedding: queryEmbedding,
    limit,
  });
}

export async function deletePage(projectPath: string, pageId: string): Promise<void> {
  await apiPost('/vector/delete', { project_path: projectPath, page_id: pageId });
}

export async function countChunks(projectPath: string): Promise<number> {
  const result = await apiGet<{ count: number }>('/vector/count', { project_path: projectPath });
  return result.count;
}