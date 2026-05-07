/**
 * File system API - replaces Tauri fs commands.
 */

import { apiGet, apiPost, apiDelete } from './client';
import type { FileNode } from '@/types/wiki';

interface ReadFileResponse {
  content: string;
}

export async function readFile(path: string): Promise<string> {
  const result = await apiGet<ReadFileResponse>('/fs/read', { path });
  return result.content;
}

export async function writeFile(path: string, contents: string): Promise<void> {
  await apiPost('/fs/write', { path, content: contents });
}

export async function listDirectory(path: string): Promise<FileNode[]> {
  return apiGet<FileNode[]>('/fs/list', { path });
}

export async function copyFile(source: string, destination: string): Promise<void> {
  await apiPost('/fs/copy', { source, destination });
}

export async function copyDirectory(source: string, destination: string): Promise<string[]> {
  return apiPost<string[]>('/fs/copy-dir', { source, destination });
}

export async function preprocessFile(path: string): Promise<string> {
  return apiPost<string>('/fs/preprocess', { path });
}

export async function deleteFile(path: string): Promise<void> {
  await apiDelete('/fs/delete', { path });
}

export async function createDirectory(path: string): Promise<void> {
  await apiPost('/fs/create-dir', { path });
}

export async function fileExists(path: string): Promise<boolean> {
  return apiGet<boolean>('/fs/exists', { path });
}

interface FileBase64 {
  base64: string;
  mimeType: string;
}

export async function readFileAsBase64(path: string): Promise<FileBase64> {
  return apiGet<FileBase64>('/fs/read-base64', { path });
}

export async function findRelatedWikiPages(projectPath: string, sourceName: string): Promise<string[]> {
  return apiGet<string[]>('/wiki/related', { project_path: projectPath, source_name: sourceName });
}

interface UploadedFile {
  name: string;
  path: string;
  size: number;
}

/**
 * Upload files to the server via multipart/form-data.
 * Files are saved into the `destination` directory on the server.
 * For folder uploads, each file's relative path (webkitRelativePath)
 * is sent alongside the file to preserve directory structure.
 */
export async function uploadFiles(files: File[], destination: string): Promise<UploadedFile[]> {
  const formData = new FormData();
  formData.append('destination', destination);

  for (const file of files) {
    const relativePath = (file as { webkitRelativePath?: string }).webkitRelativePath || file.name;
    formData.append('files', file, relativePath);
  }

  const response = await fetch('/api/fs/upload', {
    method: 'POST',
    body: formData,
    // Don't set Content-Type — browser sets it automatically with boundary for multipart
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(`Upload failed: ${response.status} - ${text}`);
  }

  return response.json() as Promise<UploadedFile[]>;
}