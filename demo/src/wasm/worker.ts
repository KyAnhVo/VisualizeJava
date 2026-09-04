/// <reference lib="webworker" />

import { buildProjectGraph } from '@/lib/graph-model'
import type { WasmGraph } from '@/types/wasm-graph'
import init, { ProjectBuilder } from '@/pkg/visualize_java.js'
// `?url` keeps Vite from trying to parse the binary; the glue fetches it.
import wasmUrl from '@/pkg/visualize_java_bg.wasm?url'
import type { WorkerRequest, WorkerResponse } from './protocol'

/**
 * Parsing runs here so the main thread never blocks: a multi-thousand-file
 * project would otherwise freeze the tab for seconds.
 *
 * Layout is deliberately *not* here — see `lib/layout.ts`. ELK's engine
 * detects a worker context and installs itself as that worker's `onmessage`
 * handler rather than exporting anything, so importing it here would both
 * fail to yield a constructor and clobber this worker's own handler. It gets
 * a worker of its own instead.
 */

let wasmReady: Promise<unknown> | null = null

function ensureWasm(): Promise<unknown> {
  wasmReady ??= init({ module_or_path: wasmUrl })
  return wasmReady
}

function post(message: WorkerResponse) {
  self.postMessage(message)
}

/** Errors from wasm-bindgen arrive as bare strings, not `Error` instances. */
function describe(error: unknown): string {
  if (typeof error === 'string') return error
  if (error instanceof Error) return error.message
  return String(error)
}

/** Report at most ~50 times, so progress is smooth without flooding the bus. */
function progressStep(total: number): number {
  return Math.max(1, Math.floor(total / 50))
}

async function build(id: number, files: { path: string; file: File }[]) {
  post({ id, type: 'progress', phase: 'init', done: 0, total: files.length })
  await ensureWasm()

  const builder = new ProjectBuilder()
  try {
    const step = progressStep(files.length)
    for (let i = 0; i < files.length; i++) {
      const { path, file } = files[i]
      const bytes = new Uint8Array(await file.arrayBuffer())
      try {
        builder.add_file(path, bytes)
      } catch (error) {
        // One bad file aborts the whole import, by design. Naming the file
        // is the only way the user can act on it.
        post({ id, type: 'failed', message: describe(error), path })
        return
      }
      if (i % step === 0) {
        post({
          id,
          type: 'progress',
          phase: 'parsing',
          done: i,
          total: files.length,
        })
      }
    }

    post({
      id,
      type: 'progress',
      phase: 'building',
      done: files.length,
      total: files.length,
    })

    const raw = builder.build_graph() as WasmGraph
    post({
      id,
      type: 'built',
      graph: buildProjectGraph(raw),
      parsedFiles: builder.file_count,
    })
  } catch (error) {
    post({ id, type: 'failed', message: describe(error), path: null })
  } finally {
    // The builder owns wasm-side memory that GC will not reclaim on its own.
    builder.free()
  }
}

self.onmessage = (event: MessageEvent<WorkerRequest>) => {
  const request = event.data
  if (request.type === 'build') void build(request.id, request.files)
}
