/**
 * Web search API - calls server-side DuckDuckGo search.
 */

import { apiPost } from './client';

export interface WebSearchResult {
  title: string;
  url: string;
  snippet: string;
}

export async function webSearch(query: string, limit?: number): Promise<WebSearchResult[]> {
  const response = await apiPost<{ results: WebSearchResult[]; error?: string }>('/search/web', {
    query,
    limit: limit ?? 5,
  });

  if (response.error) {
    throw new Error(response.error);
  }

  return response.results ?? [];
}