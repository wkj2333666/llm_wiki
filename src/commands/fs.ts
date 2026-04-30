// File system API wrapper - now uses HTTP API instead of Tauri
import {
  readFile as apiReadFile,
  writeFile as apiWriteFile,
  listDirectory as apiListDirectory,
  copyFile as apiCopyFile,
  copyDirectory as apiCopyDirectory,
  preprocessFile as apiPreprocessFile,
  deleteFile as apiDeleteFile,
  createDirectory as apiCreateDirectory,
  fileExists as apiFileExists,
  readFileAsBase64 as apiReadFileAsBase64,
  findRelatedWikiPages as apiFindRelatedWikiPages,
} from "@/api/fs"
import { createProject as apiCreateProject, openProject as apiOpenProject, listProjects as apiListProjects } from "@/api/project"
import { clipServerStatus as apiClipServerStatus } from "@/api/clip"
import { ensureProjectId, upsertProjectInfo } from "@/lib/project-identity"
import type { FileNode, WikiProject } from "@/types/wiki"

export async function readFile(path: string): Promise<string> {
  return apiReadFile(path)
}

export async function writeFile(path: string, contents: string): Promise<void> {
  return apiWriteFile(path, contents)
}

export async function listDirectory(path: string): Promise<FileNode[]> {
  return apiListDirectory(path)
}

export async function copyFile(source: string, destination: string): Promise<void> {
  return apiCopyFile(source, destination)
}

export async function copyDirectory(source: string, destination: string): Promise<string[]> {
  return apiCopyDirectory(source, destination)
}

export async function preprocessFile(path: string): Promise<string> {
  return apiPreprocessFile(path)
}

export async function deleteFile(path: string): Promise<void> {
  return apiDeleteFile(path)
}

export async function createDirectory(path: string): Promise<void> {
  return apiCreateDirectory(path)
}

export async function fileExists(path: string): Promise<boolean> {
  return apiFileExists(path)
}

export interface FileBase64 {
  base64: string
  mimeType: string
}

export async function readFileAsBase64(path: string): Promise<FileBase64> {
  return apiReadFileAsBase64(path)
}

export async function createProject(name: string, path?: string): Promise<WikiProject> {
  const project = await apiCreateProject(name, path)
  const id = await ensureProjectId(project.path)
  await upsertProjectInfo(id, project.path, project.name)
  return { id, name: project.name, path: project.path }
}

export async function openProject(nameOrPath: string): Promise<WikiProject> {
  const project = await apiOpenProject(nameOrPath)
  const id = await ensureProjectId(project.path)
  await upsertProjectInfo(id, project.path, project.name)
  return { id, name: project.name, path: project.path }
}

export async function listProjects(): Promise<{ name: string; path: string; has_wiki: boolean }[]> {
  return apiListProjects()
}

export async function clipServerStatus(): Promise<string> {
  return apiClipServerStatus()
}

export async function findRelatedWikiPages(projectPath: string, sourceName: string): Promise<string[]> {
  return apiFindRelatedWikiPages(projectPath, sourceName)
}