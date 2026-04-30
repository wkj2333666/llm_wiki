/**
 * Clip server status API.
 */

import { apiGet } from './client';

export async function clipServerStatus(): Promise<string> {
  const result = await apiGet<{ status: string }>('/clip/status');
  return result.status;
}