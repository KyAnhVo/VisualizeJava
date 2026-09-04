import type { ProjectGraph } from '@/types/graph'

/** A file collected from the dropped/selected directory, with its path relative to that directory. */
export interface UploadedFile {
  path: string
  file: File
}

export type WorkerRequest = { id: number; type: 'build'; files: UploadedFile[] }

export type BuildPhase = 'init' | 'parsing' | 'building'

export type WorkerResponse =
  | { id: number; type: 'progress'; phase: BuildPhase; done: number; total: number }
  | { id: number; type: 'built'; graph: ProjectGraph; parsedFiles: number }
  /** `path` names the offending file when the failure is attributable to one. */
  | { id: number; type: 'failed'; message: string; path: string | null }
