/**
 * Claude CLI API with SSE streaming - replaces Tauri claude-cli commands.
 */

import { getAuthToken } from './client';

interface ClaudeMessage {
  role: string;
  content: string;
}

interface DetectResult {
  installed: boolean;
  version?: string;
  path?: string;
  error?: string;
}

export async function detectClaudeCli(): Promise<DetectResult> {
  const token = getAuthToken();
  const response = await fetch('/api/claude/detect', {
    headers: { Authorization: `Bearer ${token}` },
  });
  return response.json();
}

/**
 * Spawn Claude CLI and stream output via SSE.
 * Returns an EventSource-like interface for consuming the stream.
 */
export function streamClaude(
  streamId: string,
  model: string,
  messages: ClaudeMessage[],
  onLine: (line: string) => void,
  onError: (error: string) => void,
  onComplete: (code: number | null, stderr: string) => void
): () => void {
  const token = getAuthToken();

  // Use fetch + ReadableStream for POST with SSE-like response
  fetch('/api/claude/spawn', {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ stream_id: streamId, model, messages }),
  })
    .then((response) => {
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const reader = response.body?.getReader();
      if (!reader) {
        throw new Error('No response body');
      }

      const decoder = new TextDecoder();
      let buffer = '';

      // Read stream chunks
      const pump = (): Promise<void> =>
        reader.read().then(({ done, value }) => {
          if (done) {
            onComplete(null, '');
            return;
          }

          buffer += decoder.decode(value, { stream: true });

          // Split on newlines and process each SSE event
          const lines = buffer.split('\n');
          buffer = lines.pop() || ''; // Keep incomplete line in buffer

          for (const line of lines) {
            if (line.startsWith('data:')) {
              const data = line.slice(5).trim();
              if (data) {
                onLine(data);
              }
            }
          }

          return pump();
        });

      return pump();
    })
    .catch((err) => {
      onError(err.message);
    });

  // Return a cancel function (currently not implemented - would need server-side kill)
  return () => {
    // Could call /api/claude/kill here
  };
}

export async function killClaude(streamId: string): Promise<void> {
  const token = getAuthToken();
  await fetch('/api/claude/kill', {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ stream_id: streamId }),
  });
}