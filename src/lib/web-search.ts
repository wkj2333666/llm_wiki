import type { SearchApiConfig } from "@/stores/wiki-store"
import { getHttpFetch, isFetchNetworkError } from "@/lib/tauri-fetch"
import { webSearch as apiWebSearch } from "@/api/search"

export interface WebSearchResult {
  title: string
  url: string
  snippet: string
  source: string
}

export async function webSearch(
  query: string,
  config: SearchApiConfig,
  maxResults: number = 10,
): Promise<WebSearchResult[]> {
  // Use server-side DuckDuckGo search when provider is duckduckgo or no API key
  if (config.provider === "duckduckgo" || config.provider === "none" || !config.apiKey) {
    return serverSearch(query, maxResults)
  }

  switch (config.provider) {
    case "tavily":
      return tavilySearch(query, config.apiKey, maxResults)
    default:
      // Fall back to server search for unknown providers
      return serverSearch(query, maxResults)
  }
}

async function serverSearch(query: string, maxResults: number): Promise<WebSearchResult[]> {
  const results = await apiWebSearch(query, maxResults)
  return results.map((r) => ({
    title: r.title ?? "无标题",
    url: r.url ?? "",
    snippet: r.snippet ?? "",
    source: r.url ? new URL(r.url).hostname.replace("www.", "") : "",
  }))
}

async function tavilySearch(
  query: string,
  apiKey: string,
  maxResults: number,
): Promise<WebSearchResult[]> {
  // Route through the Tauri HTTP plugin so future non-Tavily search
  // providers (Serper, Exa, Brave, Google CSE, ...) with less friendly
  // CORS don't each need their own workaround. See tauri-fetch.ts.
  const httpFetch = await getHttpFetch()
  let response: Response
  try {
    response = await httpFetch("https://api.tavily.com/search", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        api_key: apiKey,
        query,
        max_results: maxResults,
        search_depth: "advanced",
        include_answer: false,
      }),
    })
  } catch (err) {
    if (isFetchNetworkError(err)) {
      throw new Error(
        "Network error reaching api.tavily.com. Check your connectivity and whether the Tavily API key is still valid.",
      )
    }
    throw err
  }

  if (!response.ok) {
    const errorText = await response.text().catch(() => "Unknown error")
    throw new Error(`Tavily search failed (${response.status}): ${errorText}`)
  }

  const data = await response.json()

  return (data.results ?? []).map((r: { title: string; url: string; content: string }) => ({
    title: r.title ?? "无标题",
    url: r.url ?? "",
    snippet: r.content ?? "",
    source: new URL(r.url).hostname.replace("www.", ""),
  }))
}
