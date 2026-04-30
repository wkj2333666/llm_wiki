/**
 * Shared HTTP helpers.
 *
 * In web server mode, all requests go directly through fetch.
 * CORS is handled by the server or the target endpoint directly.
 */

export function getHttpFetch(): Promise<typeof globalThis.fetch> {
  return Promise.resolve(globalThis.fetch.bind(globalThis))
}

export function isFetchNetworkError(err: unknown): boolean {
  if (!(err instanceof Error)) return false
  if (err.name === "AbortError") return false
  if (err.name === "TypeError") return true
  if (err.message === "Load failed") return true
  if (err.message === "Failed to fetch") return true
  if (err.message.includes("network error")) return true
  return false
}