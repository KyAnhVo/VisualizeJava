import type {
  BuildPhase,
  UploadedFile,
  WorkerRequest,
  WorkerResponse,
} from './protocol'
import type { ProjectGraph } from '@/types/graph'

export interface BuildProgress {
  phase: BuildPhase
  done: number
  total: number
}

export interface BuildResult {
  graph: ProjectGraph
  parsedFiles: number
}

/** A failure attributable to a specific file carries its path. */
export class BuildFailure extends Error {
  readonly path: string | null

  constructor(message: string, path: string | null) {
    super(message)
    this.name = 'BuildFailure'
    this.path = path
  }
}

/**
 * `Omit` over a union collapses it to the members' shared keys, which would
 * erase the request payloads. Distributing keeps each variant intact.
 */
type DistributiveOmit<T, K extends PropertyKey> = T extends unknown
  ? Omit<T, K>
  : never

interface Pending {
  resolve: (value: never) => void
  reject: (error: Error) => void
  onProgress?: (progress: BuildProgress) => void
}

/**
 * Promise-shaped wrapper over the parsing worker. Requests are
 * correlated by an incrementing id, so several can be in flight at once.
 */
class GraphWorkerClient {
  private worker: Worker | null = null
  private nextId = 1
  private readonly pending = new Map<number, Pending>()

  private ensureWorker(): Worker {
    if (!this.worker) {
      this.worker = new Worker(new URL('./worker.ts', import.meta.url), {
        type: 'module',
      })
      this.worker.onmessage = (event: MessageEvent<WorkerResponse>) =>
        this.handle(event.data)
      this.worker.onerror = (event) => this.failAll(event.message)
    }
    return this.worker
  }

  private handle(response: WorkerResponse) {
    const pending = this.pending.get(response.id)
    if (!pending) return

    switch (response.type) {
      case 'progress':
        pending.onProgress?.({
          phase: response.phase,
          done: response.done,
          total: response.total,
        })
        break
      case 'built':
        this.pending.delete(response.id)
        pending.resolve({
          graph: response.graph,
          parsedFiles: response.parsedFiles,
        } as never)
        break
      case 'failed':
        this.pending.delete(response.id)
        pending.reject(new BuildFailure(response.message, response.path))
        break
    }
  }

  /**
   * A worker-level error (module failed to load, wasm trap) leaves every
   * in-flight request unanswered, so reject them all and start clean.
   */
  private failAll(message: string) {
    for (const pending of this.pending.values()) {
      pending.reject(new BuildFailure(message, null))
    }
    this.pending.clear()
    this.worker?.terminate()
    this.worker = null
  }

  private send<T>(
    request: DistributiveOmit<WorkerRequest, 'id'>,
    onProgress?: (progress: BuildProgress) => void,
  ): Promise<T> {
    const worker = this.ensureWorker()
    const id = this.nextId++
    return new Promise<T>((resolve, reject) => {
      this.pending.set(id, {
        resolve: resolve as (value: never) => void,
        reject,
        onProgress,
      })
      worker.postMessage({ ...request, id } as WorkerRequest)
    })
  }

  build(
    files: UploadedFile[],
    onProgress?: (progress: BuildProgress) => void,
  ): Promise<BuildResult> {
    return this.send<BuildResult>({ type: 'build', files }, onProgress)
  }
}

/**
 * Single shared worker for the app. Spinning one up per component would pay
 * the wasm instantiation cost repeatedly, and React's StrictMode double-mount
 * would do it twice on every mount in development.
 */
export const graphWorker = new GraphWorkerClient()
