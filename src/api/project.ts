/**
 * Project API - replaces Tauri project commands.
 */

import { apiPost } from './client';
import type { WikiProject } from '@/types/wiki';

export async function createProject(name: string, path: string): Promise<WikiProject> {
  return apiPost<WikiProject>('/project/create', { name, path });
}

export async function openProject(path: string): Promise<WikiProject> {
  return apiPost<WikiProject>('/project/open', { path });
}