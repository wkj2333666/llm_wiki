/**
 * Config API - replaces tauri-plugin-store.
 */

import { apiGet, apiPost, apiDelete } from './client';
import type { WikiProject } from '@/types/wiki';

export async function getConfig(key: string): Promise<unknown> {
  return apiGet('/config/get', { key });
}

export async function setConfig(key: string, value: unknown): Promise<void> {
  await apiPost('/config/set', { key, value });
}

export async function getProjects(): Promise<WikiProject[]> {
  return apiGet<WikiProject[]>('/config/projects');
}

export async function getLastProject(): Promise<WikiProject | null> {
  return apiGet<WikiProject | null>('/config/last-project');
}

export async function addProject(project: WikiProject): Promise<void> {
  await apiPost('/config/projects/add', { project });
}

export async function removeProject(path: string): Promise<void> {
  await apiDelete('/config/projects/remove', { path });
}

/**
 * Get server-side configuration (from server.toml).
 * This returns LLM, embedding, and search config that was set in the config file.
 */
export async function getServerConfig(): Promise<ServerConfigResponse> {
  return apiGet<ServerConfigResponse>('/config/server', {});
}

interface ServerConfigResponse {
  llm: {
    provider: string;
    apiKey: string;
    model: string;
    maxContextSize: number;
    ollamaUrl: string;
    customEndpoint: string;
    apiMode: string;
  };
  embedding: {
    provider: string;
    apiKey: string;
    model: string;
  };
  search: {
    enabled: boolean;
    provider: string;
    apiKey: string;
  };
}