/**
 * Image extraction orchestration for the ingest pipeline.
 *
 * Pure dispatch + path-shaping layer over the HTTP API.
 * Decides which endpoint to call based on file extension, computes the
 * destination directory (`wiki/media/<source-slug>/`), and gives back
 * a small markdown snippet ready to paste into the LLM's source context.
 */
import { apiPost } from "@/api/client"
import { getFileName, normalizePath } from "@/lib/path-utils"

/** Saved image metadata from extraction. */
export interface SavedImage {
  index: number
  mimeType: string
  /** PDF page or PPTX slide number (1-based). DOCX always null. */
  page: number | null
  width: number
  height: number
  /** Path relative to the wiki/ root, e.g. `media/rope-paper/img-1.png`. */
  relPath: string
  /** Absolute filesystem path — used for preview. */
  absPath: string
  sha256: string
}

const SUPPORTED_PDF_EXTS = ["pdf"] as const
const SUPPORTED_OFFICE_EXTS = ["pptx", "docx", "ppt", "doc"] as const

/**
 * Extract every embedded image from `sourcePath` and save them to
 * `<projectPath>/wiki/media/<slug>/`. Returns metadata only.
 *
 * Returns `[]` for unsupported file types or when the source has no
 * extractable images. Errors during extraction are logged and returned
 * as an empty array — image extraction failure must NEVER abort the
 * ingest pipeline.
 */
export async function extractAndSaveSourceImages(
  projectPath: string,
  sourcePath: string,
): Promise<SavedImage[]> {
  const pp = normalizePath(projectPath)
  const sp = normalizePath(sourcePath)
  const fileName = getFileName(sp)
  const ext = fileName.split(".").pop()?.toLowerCase() ?? ""

  const isPdf = (SUPPORTED_PDF_EXTS as readonly string[]).includes(ext)
  const isOffice = (SUPPORTED_OFFICE_EXTS as readonly string[]).includes(ext)
  if (!isPdf && !isOffice) return []

  const slug = fileName.replace(/\.[^.]+$/, "")
  const destDir = `${pp}/wiki/media/${slug}`
  const relTo = `${pp}/wiki`

  try {
    const images = await apiPost<unknown[]>(
      "/fs/extract-images",
      { sourcePath: sp, destDir, relTo, isPdf },
    )
    return images
      .filter((it): it is SavedImage => {
        if (!it || typeof it !== "object") return false
        const obj = it as Record<string, unknown>
        return (
          typeof obj.index === "number" &&
          typeof obj.relPath === "string" &&
          typeof obj.absPath === "string"
        )
      })
  } catch (err) {
    console.warn(
      `[ingest:images] extraction failed for "${fileName}":`,
      err instanceof Error ? err.message : err,
    )
    return []
  }
}

/**
 * Given a list of saved images, return a markdown snippet that
 * references each one. The snippet is suitable for pasting into
 * the LLM's source context.
 */
export function imagesMarkdownSnippet(images: SavedImage[]): string {
  if (images.length === 0) return ""

  const lines = images.map((img) => {
    const alt = img.page ? `Image from page ${img.page}` : `Image ${img.index + 1}`
    return `![${alt}](${img.relPath})`
  })

  return `\n\n---\n\n### Embedded Images\n\n${lines.join("\n")}\n`
}

/**
 * Build the markdown section to splice into `sourceContent` so the
 * generation LLM sees the available images. Each image is referenced
 * once by its rel_path with caption text as alt.
 *
 * Returns an empty string when there are no images — no leading
 * separator gets inserted, which keeps the prompt size unchanged for
 * pure-text documents.
 */
export function buildImageMarkdownSection(
  images: SavedImage[],
  captionsBySha?: Map<string, string>,
): string {
  if (images.length === 0) return ""

  const lines: string[] = ["", "", "## Embedded Images", ""]
  // Group by page so the LLM can correlate "Figure 3 mentioned on
  // page 5" with the right image. DOCX images have page=null; they
  // get grouped under "Document":
  const byPage = new Map<string, SavedImage[]>()
  for (const img of images) {
    const key = img.page == null ? "Document" : `Page ${img.page}`
    const bucket = byPage.get(key)
    if (bucket) bucket.push(img)
    else byPage.set(key, [img])
  }

  // Page-keyed order, with "Document" (DOCX) last when present.
  const ordered = [...byPage.keys()].sort((a, b) => {
    if (a === "Document") return 1
    if (b === "Document") return -1
    const numA = parseInt(a.replace(/\D/g, ""), 10) || 0
    const numB = parseInt(b.replace(/\D/g, ""), 10) || 0
    return numA - numB
  })

  // Sanitize a caption for safe inclusion as alt text — the same
  // rules as the inline-rewrite path: no `]` (would close the alt
  // bracket early), no embedded newlines (would break the markdown
  // image syntax across lines).
  const sanitize = (s: string): string =>
    s.replace(/[\r\n]+/g, " ").replace(/]/g, ")").trim()

  for (const key of ordered) {
    lines.push(`### ${key}`, "")
    for (const img of byPage.get(key) ?? []) {
      // Caption lookup by SHA-256 — same key the caption pipeline
      // uses to dedupe across documents. Falling back to empty alt
      // text if no caption is available for this image.
      const caption = captionsBySha?.get(img.sha256)
      const alt = caption ? sanitize(caption) : ""
      lines.push(`![${alt}](${img.relPath})`)
    }
    lines.push("")
  }

  return lines.join("\n")
}