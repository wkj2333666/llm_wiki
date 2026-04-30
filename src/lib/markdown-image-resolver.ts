/**
 * Resolve markdown image `src` attributes so they load in the browser.
 *
 * In web server mode, images are served via API endpoint.
 *
 * Convention:
 *   - Any src starting with `http://`, `https://`, `data:`, `blob:` is passed through unchanged.
 *   - Any src starting with `/` (absolute) is served via `/api/fs/file?path=...`
 *   - Anything else is treated as relative to the project's `wiki/` root.
 */

import { normalizePath } from "@/lib/path-utils"
import { getAuthToken } from "@/api/client"

const PASSTHROUGH_RE = /^(https?:|data:|blob:)/i

/**
 * Convert a local file path to a URL that the browser can load.
 * In web mode, this uses the API endpoint `/api/fs/file?path=...`.
 */
function convertFileSrc(path: string): string {
  const token = getAuthToken()
  const encodedPath = encodeURIComponent(path)
  return `/api/fs/file?path=${encodedPath}&token=${token}`
}

/**
 * `projectPath` is the wiki project's root directory. When null
 * (no project loaded), the resolver passes srcs through unchanged.
 */
export function resolveMarkdownImageSrc(
  rawSrc: string,
  projectPath: string | null,
): string {
  if (!rawSrc) return rawSrc
  if (PASSTHROUGH_RE.test(rawSrc)) return rawSrc

  if (!projectPath) return rawSrc

  const pp = normalizePath(projectPath)
  const isAbsolute =
    rawSrc.startsWith("/") || /^[a-zA-Z]:/.test(rawSrc) || rawSrc.startsWith("\\\\")

  // Absolute paths get fed straight to convertFileSrc
  if (isAbsolute) return convertFileSrc(rawSrc)

  // Strip a leading `./` for cleanliness
  const cleaned = rawSrc.replace(/^\.\//, "")

  // Resolve as wiki-root-relative
  const absolute = `${pp}/wiki/${cleaned}`
  return convertFileSrc(absolute)
}