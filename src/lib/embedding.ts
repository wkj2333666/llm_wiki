/**
 * Embedding pipeline — standard RAG flow.
 *
 *   1. chunkMarkdown(content)        (src/lib/text-chunker.ts)
 *   2. for each chunk:
 *        fetchEmbedding(title + heading_path + chunk_text)
 *        with auto-halve retry on "input too long" errors
 *   3. vector_upsert_chunks(page_id, [{chunk_index, chunk_text,
 *      heading_path, embedding}, …])
 *
 * Search:
 *   1. fetchEmbedding(query)
 *   2. vector_search_chunks(query_emb, topK × 3)
 *   3. group by page_id, max-pool primary score + weighted tail sum
 *   4. return top-K pages, outer API-compatible with the old per-page
 *      `{id, score}[]` shape; matched chunks available on the
 *      optional `matchedChunks` field for future UI surfacing.
 */

import { readFile, listDirectory } from "@/commands/fs"
import type { EmbeddingConfig } from "@/stores/wiki-store"
import type { FileNode } from "@/types/wiki"
import { normalizePath } from "@/lib/path-utils"
import { chunkMarkdown, type Chunk } from "@/lib/text-chunker"
import { upsertChunks, searchChunks, deletePage, countChunks } from "@/api/vector"

// ── Error surfacing ──────────────────────────────────────────────────────

let lastEmbeddingError: string | null = null

export function getLastEmbeddingError(): string | null {
  return lastEmbeddingError
}

// ── fetchEmbedding with auto-halve retry ────────────────────────────────

export function looksLikeOversizeError(httpStatus: number, body: string): boolean {
  if (httpStatus === 413) return true
  const lower = body.toLowerCase()
  return (
    lower.includes("too long") ||
    lower.includes("maximum context") ||
    lower.includes("max_tokens") ||
    lower.includes("max tokens") ||
    lower.includes("context length") ||
    lower.includes("token limit") ||
    lower.includes("exceeds") ||
    lower.includes("input length")
  )
}

export async function fetchEmbedding(
  text: string,
  cfg: EmbeddingConfig,
  maxRetries = 3,
): Promise<number[] | null> {
  if (!cfg.endpoint) return null

  const headers: Record<string, string> = { "Content-Type": "application/json" }
  if (cfg.apiKey) headers.Authorization = `Bearer ${cfg.apiKey}`

  let current = text
  let attempts = 0
  while (attempts <= maxRetries) {
    attempts++
    try {
      const resp = await fetch(cfg.endpoint, {
        method: "POST",
        headers,
        body: JSON.stringify({ model: cfg.model, input: current }),
      })

      if (resp.ok) {
        const data = await resp.json()
        const embedding = data?.data?.[0]?.embedding ?? null
        if (embedding) {
          lastEmbeddingError = null
          return embedding
        }
        lastEmbeddingError = `Embedding response missing data[0].embedding (got ${JSON.stringify(data).slice(0, 200)})`
        console.warn(`[Embedding] ${lastEmbeddingError}`)
        return null
      }

      let bodyText = ""
      try {
        bodyText = await resp.text()
      } catch {
        // ignore
      }

      if (looksLikeOversizeError(resp.status, bodyText)) {
        if (current.length > 64 && attempts <= maxRetries) {
          const prev = current.length
          current = current.slice(0, Math.floor(current.length / 2))
          console.warn(
            `[Embedding] auto-halving after HTTP ${resp.status} at ${prev} chars → retrying at ${current.length} chars (attempt ${attempts}/${maxRetries + 1})`,
          )
          continue
        }
        lastEmbeddingError = `Endpoint rejected input even at ${current.length} chars — server context smaller than expected. Lower Settings → Embedding → Max Chunk Chars (${bodyText.slice(0, 160)}).`
        console.warn(`[Embedding] ${lastEmbeddingError}`)
        return null
      }

      lastEmbeddingError = `API ${resp.status} ${resp.statusText}${bodyText ? ` — ${bodyText.slice(0, 200)}` : ""} at ${cfg.endpoint}`
      console.warn(`[Embedding] ${lastEmbeddingError}`)
      return null
    } catch (err) {
      lastEmbeddingError = err instanceof Error ? err.message : String(err)
      console.warn(`[Embedding] ${lastEmbeddingError}`)
      return null
    }
  }

  lastEmbeddingError = `Embedding endpoint rejected every size down to ${current.length} chars — the server's context is smaller than ${current.length * 2}. Lower Settings → Embedding → Max Chunk Chars.`
  console.warn(`[Embedding] ${lastEmbeddingError}`)
  return null
}

// ── Vector operations via HTTP API ───────────────────────────────────────

interface ChunkUpsertInput {
  chunkIndex: number
  chunkText: string
  headingPath: string
  embedding: number[]
}

interface ChunkSearchResult {
  chunk_id: string
  page_id: string
  chunk_index: number
  chunk_text: string
  heading_path: string
  score: number
}

async function vectorUpsertChunks(
  projectPath: string,
  pageId: string,
  chunks: ChunkUpsertInput[],
): Promise<void> {
  await upsertChunks(normalizePath(projectPath), pageId, chunks.map((c) => ({
    chunk_index: c.chunkIndex,
    chunk_text: c.chunkText,
    heading_path: c.headingPath,
    embedding: c.embedding.map((v) => Math.fround(v)),
  })))
}

async function vectorSearchChunks(
  projectPath: string,
  queryEmbedding: number[],
  topK: number,
): Promise<ChunkSearchResult[]> {
  return await searchChunks(normalizePath(projectPath), queryEmbedding.map((v) => Math.fround(v)), topK)
}

async function vectorDeletePage(projectPath: string, pageId: string): Promise<void> {
  await deletePage(normalizePath(projectPath), pageId)
}

async function vectorCountChunks(projectPath: string): Promise<number> {
  return await countChunks(normalizePath(projectPath))
}

export async function legacyVectorRowCount(_projectPath: string): Promise<number> {
  return 0
}

export async function dropLegacyVectorTable(_projectPath: string): Promise<void> {
  // No longer needed
}

// ── Chunk enrichment ─────────────────────────────────────────────────────

function enrichChunkForEmbedding(
  pageTitle: string,
  chunk: Chunk,
): string {
  const parts: string[] = []
  if (pageTitle.trim().length > 0) parts.push(pageTitle.trim())
  if (chunk.headingPath.trim().length > 0) parts.push(chunk.headingPath.trim())
  parts.push(chunk.text.trim())
  return parts.join("\n\n")
}

// ── Public API: embedPage / embedAllPages / searchByEmbedding ────────────

export async function embedPage(
  projectPath: string,
  pageId: string,
  title: string,
  content: string,
  cfg: EmbeddingConfig,
): Promise<void> {
  if (!cfg.enabled || !cfg.model) return

  const t0 = performance.now()
  const chunks = chunkMarkdown(content, {
    targetChars: cfg.maxChunkChars ?? 1000,
    overlapChars: cfg.overlapChunkChars ?? 200,
  })
  if (chunks.length === 0) return

  const rows: ChunkUpsertInput[] = []
  let failedChunks = 0
  for (const chunk of chunks) {
    const embedText = enrichChunkForEmbedding(title, chunk)
    const vec = await fetchEmbedding(embedText, cfg)
    if (vec) {
      rows.push({
        chunkIndex: chunk.index,
        chunkText: chunk.text,
        headingPath: chunk.headingPath,
        embedding: vec,
      })
    } else {
      failedChunks++
    }
  }

  if (rows.length === 0) {
    console.log(
      `[Embedding] Indexed nothing for "${pageId}" — all ${chunks.length} chunks failed. See getLastEmbeddingError().`,
    )
    return
  }

  await vectorUpsertChunks(projectPath, pageId, rows)
  const elapsed = Math.round(performance.now() - t0)
  console.log(
    `[Embedding] Indexed "${pageId}": ${rows.length}/${chunks.length} chunks (${failedChunks} skipped) in ${elapsed}ms`,
  )
}

export async function embedAllPages(
  projectPath: string,
  cfg: EmbeddingConfig,
  onProgress?: (done: number, total: number) => void,
): Promise<number> {
  if (!cfg.enabled || !cfg.model) return 0

  const pp = normalizePath(projectPath)

  let tree: FileNode[]
  try {
    tree = await listDirectory(`${pp}/wiki`)
  } catch {
    return 0
  }

  const mdFiles: { id: string; path: string }[] = []
  function walk(nodes: FileNode[]) {
    for (const node of nodes) {
      if (node.is_dir && node.children) {
        walk(node.children)
      } else if (!node.is_dir && node.name.endsWith(".md")) {
        const id = node.name.replace(/\.md$/, "")
        if (!["index", "log", "overview", "purpose", "schema"].includes(id)) {
          mdFiles.push({ id, path: node.path })
        }
      }
    }
  }
  walk(tree)

  let done = 0
  for (const file of mdFiles) {
    try {
      const content = await readFile(file.path)
      const titleMatch = content.match(/^---\n[\s\S]*?^title:\s*["']?(.+?)["']?\s*$/m)
      const title = titleMatch ? titleMatch[1].trim() : file.id
      await embedPage(pp, file.id, title, content, cfg)
    } catch {
      // skip
    }
    done++
    if (onProgress) onProgress(done, mdFiles.length)
  }

  return done
}

export interface PageSearchResult {
  id: string
  score: number
  matchedChunks?: Array<{ text: string; headingPath: string; score: number }>
}

export async function searchByEmbedding(
  projectPath: string,
  query: string,
  cfg: EmbeddingConfig,
  topK: number = 10,
): Promise<PageSearchResult[]> {
  if (!cfg.enabled || !cfg.model) return []

  const queryEmb = await fetchEmbedding(query, cfg)
  if (!queryEmb) return []

  const t0 = performance.now()
  let rawChunks: ChunkSearchResult[] = []
  try {
    rawChunks = await vectorSearchChunks(projectPath, queryEmb, Math.max(topK * 3, 30))
  } catch (err) {
    console.log(`[Embedding] LanceDB chunk search failed: ${err instanceof Error ? err.message : err}`)
    return []
  }
  if (rawChunks.length === 0) return []

  const byPage = new Map<string, ChunkSearchResult[]>()
  for (const c of rawChunks) {
    const bucket = byPage.get(c.page_id)
    if (bucket) bucket.push(c)
    else byPage.set(c.page_id, [c])
  }

  const ranked: PageSearchResult[] = []
  for (const [pageId, chunks] of byPage.entries()) {
    chunks.sort((a, b) => b.score - a.score)
    const top = chunks[0].score
    const tail = chunks.slice(1).reduce((sum, c) => sum + c.score, 0)
    const blended = top + Math.min(tail * 0.3, Math.max(0, 1 - top))
    ranked.push({
      id: pageId,
      score: blended,
      matchedChunks: chunks.slice(0, 3).map((c) => ({
        text: c.chunk_text,
        headingPath: c.heading_path,
        score: c.score,
      })),
    })
  }
  ranked.sort((a, b) => b.score - a.score)

  const elapsed = Math.round(performance.now() - t0)
  console.log(
    `[Embedding] LanceDB chunk search: ${rawChunks.length} chunks → ${ranked.length} pages in ${elapsed}ms`,
  )

  return ranked.slice(0, topK)
}

export async function removePageEmbedding(
  projectPath: string,
  pageId: string,
): Promise<void> {
  try {
    await vectorDeletePage(projectPath, pageId)
  } catch {
    // non-critical
  }
}

export async function getEmbeddingCount(projectPath: string): Promise<number> {
  try {
    return await vectorCountChunks(projectPath)
  } catch {
    return 0
  }
}