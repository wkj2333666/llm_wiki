import { apiGet, apiPost, apiPut, apiDelete } from './client';

export interface User {
  id: string;
  username: string;
  role: string;
  created_at: number;
}

export async function listUsers(): Promise<User[]> {
  return apiGet<User[]>('/admin/users');
}

export async function createUser(username: string, password: string, role: string): Promise<User> {
  return apiPost<User>('/admin/users', { username, password, role });
}

export async function updateUser(username: string, patch: { password?: string; role?: string }): Promise<User> {
  return apiPut<User>('/admin/users', { username, ...patch });
}

export async function deleteUser(username: string): Promise<void> {
  return apiDelete<void>('/admin/users', { username });
}

export async function assignProject(projectId: string, userId: string): Promise<void> {
  return apiPost<void>('/admin/projects/assign', { project_id: projectId, user_id: userId });
}
