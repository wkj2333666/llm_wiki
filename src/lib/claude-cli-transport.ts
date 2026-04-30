/**
 * Claude Code CLI subprocess transport via HTTP API.
 *
 * The server spawns `claude -p --output-format stream-json --input-format stream-json --verbose --model <model>`,
 * pipes the serialized history over stdin, and emits stdout back as SSE events.
 */

import { streamClaude, killClaude } from "@/api/claude"
import type { LlmConfig } from "@/stores/wiki-store"
import type { ChatMessage, RequestOverrides } from "./llm-providers"
import type { StreamCallbacks } from "./llm-client"

/**
 * Public parse entry point. Given one stream-json line from claude's
 * stdout, returns any assistant text it contains.
 */
export function createClaudeCodeStreamParser() {
  let sawDelta = false
  let emittedFromAssistant = ""

  return function parseLine(rawLine: string): string | null {
    const line = rawLine.trim()
    if (!line) return null

    let evt: unknown
    try {
      evt = JSON.parse(line)
    } catch {
      return null
    }

    if (!evt || typeof evt !== "object") return null
    const obj = evt as Record<string, unknown>
    const type = obj.type

    if (type === "stream_event") {
      const event = obj.event as Record<string, unknown> | undefined
      if (event?.type === "content_block_delta") {
        const delta = event.delta as Record<string, unknown> | undefined
        if (delta?.type === "text_delta" && typeof delta.text === "string") {
          sawDelta = true
          return delta.text
        }
      }
      return null
    }

    if (type === "assistant") {
      const message = obj.message as Record<string, unknown> | undefined
      const content = message?.content
      if (!Array.isArray(content)) return null
      const text = content
        .map((c) => {
          const cc = c as Record<string, unknown>
          return cc.type === "text" && typeof cc.text === "string" ? cc.text : ""
        })
        .join("")
      if (!text) return null

      if (sawDelta) {
        return null
      }
      if (text.startsWith(emittedFromAssistant)) {
        const novel = text.slice(emittedFromAssistant.length)
        emittedFromAssistant = text
        return novel || null
      }
      emittedFromAssistant = text
      return text
    }

    return null
  }
}

/**
 * HTTP SSE equivalent of streamChat. Obeys the same StreamCallbacks contract.
 */
export async function streamClaudeCodeCli(
  config: LlmConfig,
  messages: ChatMessage[],
  callbacks: StreamCallbacks,
  signal?: AbortSignal,
  overrides?: RequestOverrides,
): Promise<void> {
  const { onToken, onDone, onError } = callbacks

  if (import.meta.env?.DEV && overrides) {
    for (const key of ["temperature", "top_p", "top_k", "max_tokens", "stop"] as const) {
      if (overrides[key] !== undefined) {
        console.warn(`[claude-code] ignoring unsupported override "${key}": CLI has no equivalent flag`)
      }
    }
  }

  const streamId = crypto.randomUUID()
  const parse = createClaudeCodeStreamParser()
  let finished = false

  const finishWith = (cb: () => void) => {
    if (finished) return
    finished = true
    cb()
  }

  const abortListener = () => {
    killClaude(streamId).catch(() => {})
    finishWith(onDone)
  }
  signal?.addEventListener("abort", abortListener)

  // Convert ChatMessage to simple string content format for Claude CLI
  const simpleMessages = messages.map((m) => ({
    role: m.role,
    content: typeof m.content === "string" ? m.content : m.content
      .filter((b) => b.type === "text")
      .map((b) => b.text)
      .join("\n"),
  }))

  try {
    streamClaude(
      streamId,
      config.model,
      simpleMessages,
      (line) => {
        const token = parse(line)
        if (token !== null) onToken(token)
      },
      (error) => {
        finishWith(() => onError(new Error(error)))
      },
      (code, stderr) => {
        if (code !== null && code !== 0) {
          const detail = stderr ? `: ${stderr}` : ""
          finishWith(() => onError(new Error(`claude CLI exited with code ${code}${detail}`)))
        } else {
          finishWith(onDone)
        }
      },
    )
  } catch (err) {
    finishWith(() => {
      const message = err instanceof Error ? err.message : String(err)
      if (/not found|No such file|executable file not found/i.test(message)) {
        onError(new Error(
          "Claude Code CLI not found. Install `claude` or pick a different provider.",
        ))
      } else {
        onError(err instanceof Error ? err : new Error(message))
      }
    })
  } finally {
    signal?.removeEventListener("abort", abortListener)
  }
}