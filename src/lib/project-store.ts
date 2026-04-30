// Config storage - now uses HTTP API instead of tauri-plugin-store
import { getConfig, setConfig } from "@/api/config"
import type { WikiProject } from "@/types/wiki"
import type { LlmConfig, SearchApiConfig, EmbeddingConfig, MultimodalConfig, OutputLanguage, ProviderConfigs } from "@/stores/wiki-store"

const RECENT_PROJECTS_KEY = "recentProjects"
const LAST_PROJECT_KEY = "lastProject"

export async function getRecentProjects(): Promise<WikiProject[]> {
  const projects = await getConfig(RECENT_PROJECTS_KEY)
  return (projects as WikiProject[]) ?? []
}

export async function getLastProject(): Promise<WikiProject | null> {
  const project = await getConfig(LAST_PROJECT_KEY)
  return (project as WikiProject) ?? null
}

export async function saveLastProject(project: WikiProject): Promise<void> {
  await setConfig(LAST_PROJECT_KEY, project)
  await addToRecentProjects(project)
}

export async function addToRecentProjects(project: WikiProject): Promise<void> {
  const existing = await getRecentProjects()
  const filtered = existing.filter((p) => p.path !== project.path)
  const updated = [project, ...filtered].slice(0, 10)
  await setConfig(RECENT_PROJECTS_KEY, updated)
}

const LLM_CONFIG_KEY = "llmConfig"
const PROVIDER_CONFIGS_KEY = "providerConfigs"
const ACTIVE_PRESET_KEY = "activePresetId"

export async function saveLlmConfig(config: LlmConfig): Promise<void> {
  await setConfig(LLM_CONFIG_KEY, config)
}

export async function loadLlmConfig(): Promise<LlmConfig | null> {
  const config = await getConfig(LLM_CONFIG_KEY)
  return (config as LlmConfig) ?? null
}

export async function saveProviderConfigs(configs: ProviderConfigs): Promise<void> {
  await setConfig(PROVIDER_CONFIGS_KEY, configs)
}

export async function loadProviderConfigs(): Promise<ProviderConfigs | null> {
  const configs = await getConfig(PROVIDER_CONFIGS_KEY)
  return (configs as ProviderConfigs) ?? null
}

export async function saveActivePresetId(id: string | null): Promise<void> {
  await setConfig(ACTIVE_PRESET_KEY, id)
}

export async function loadActivePresetId(): Promise<string | null> {
  const id = await getConfig(ACTIVE_PRESET_KEY)
  return (id as string) ?? null
}

const SEARCH_API_KEY = "searchApiConfig"

export async function saveSearchApiConfig(config: SearchApiConfig): Promise<void> {
  await setConfig(SEARCH_API_KEY, config)
}

export async function loadSearchApiConfig(): Promise<SearchApiConfig | null> {
  const config = await getConfig(SEARCH_API_KEY)
  return (config as SearchApiConfig) ?? null
}

const EMBEDDING_KEY = "embeddingConfig"

export async function saveEmbeddingConfig(config: EmbeddingConfig): Promise<void> {
  await setConfig(EMBEDDING_KEY, config)
}

export async function loadEmbeddingConfig(): Promise<EmbeddingConfig | null> {
  const config = await getConfig(EMBEDDING_KEY)
  return (config as EmbeddingConfig) ?? null
}

const MULTIMODAL_KEY = "multimodalConfig"

export async function saveMultimodalConfig(config: MultimodalConfig): Promise<void> {
  await setConfig(MULTIMODAL_KEY, config)
}

export async function loadMultimodalConfig(): Promise<MultimodalConfig | null> {
  const config = await getConfig(MULTIMODAL_KEY)
  return (config as MultimodalConfig) ?? null
}

export async function removeFromRecentProjects(path: string): Promise<void> {
  const existing = await getRecentProjects()
  const updated = existing.filter((p) => p.path !== path)
  await setConfig(RECENT_PROJECTS_KEY, updated)

  const last = await getLastProject()
  if (last && last.path === path) {
    await setConfig(LAST_PROJECT_KEY, null)
  }
}

const LANGUAGE_KEY = "language"

export async function saveLanguage(lang: string): Promise<void> {
  await setConfig(LANGUAGE_KEY, lang)
}

export async function loadLanguage(): Promise<string | null> {
  const lang = await getConfig(LANGUAGE_KEY)
  return (lang as string) ?? null
}

const OUTPUT_LANGUAGE_KEY = "outputLanguage"

export async function saveOutputLanguage(lang: OutputLanguage): Promise<void> {
  await setConfig(OUTPUT_LANGUAGE_KEY, lang)
}

export async function loadOutputLanguage(): Promise<OutputLanguage | null> {
  const lang = await getConfig(OUTPUT_LANGUAGE_KEY)
  return (lang as OutputLanguage) ?? null
}

// Update-check persistence
const UPDATE_CHECK_STATE_KEY = "updateCheckState"

export interface PersistedUpdateCheckState {
  enabled: boolean
  lastCheckedAt: number | null
  dismissedVersion: string | null
}

export async function saveUpdateCheckState(state: PersistedUpdateCheckState): Promise<void> {
  await setConfig(UPDATE_CHECK_STATE_KEY, state)
}

export async function loadUpdateCheckState(): Promise<PersistedUpdateCheckState | null> {
  const state = await getConfig(UPDATE_CHECK_STATE_KEY)
  return (state as PersistedUpdateCheckState) ?? null
}