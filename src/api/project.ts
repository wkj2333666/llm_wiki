/**
 * Project API - replaces Tauri project commands.
 */

import { apiGet, apiPost } from './client';
import type { WikiProject } from '@/types/wiki';

export interface ProjectInfo {
  name: string;
  path: string;
  has_wiki: boolean;
}

export async function listProjects(): Promise<ProjectInfo[]> {
  return apiGet<ProjectInfo[]>('/project/list');
}

export async function createProject(name: string, path?: string): Promise<WikiProject> {
  // If path is empty/undefined, server will use its configured projects_dir
  return apiPost<WikiProject>('/project/create', { name, path: path || "" });
}

/**
 * Open a project by name (for projects in projects_dir) or by path.
 * If nameOrPath contains "/" or starts with "~", treat as full path.
 * Otherwise, look up by name in projects_dir.
 */
export async function openProject(nameOrPath: string): Promise<WikiProject> {
  // Check if it's a full path (contains / or starts with ~)
  if (nameOrPath.includes("/") || nameOrPath.startsWith("~")) {
    return apiPost<WikiProject>('/project/open-path', { path: nameOrPath });
  }
  return apiPost<WikiProject>('/project/open', { name: nameOrPath });
}