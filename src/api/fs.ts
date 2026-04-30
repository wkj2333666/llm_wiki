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