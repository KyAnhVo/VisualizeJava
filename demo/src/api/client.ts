import type { RawGraph } from '@/api/types'

const API_URL = import.meta.env.VITE_API_URL ?? 'http://localhost:8080'

export const BACKEND_URL = API_URL

/** A Java source file plus the project-relative path to send it under. */
export interface UploadFile {
  file: File
  path: string
}

/** A parse failure names the file it choked on; keep it for the error panel. */
export class GraphRequestError extends Error {
  readonly status: number
  readonly detail: string

  constructor(status: number, detail: string) {
    super(detail || `Request failed with status ${status}`)
    this.name = 'GraphRequestError'
    this.status = status
    this.detail = detail
  }
}

export function isJavaFile(name: string): boolean {
  return name.endsWith('.java')
}

/** Files chosen through `<input webkitdirectory>` carry their own relative path. */
export function fromFileList(files: FileList | File[]): UploadFile[] {
  return [...files]
    .filter((file) => isJavaFile(file.name))
    .map((file) => ({ file, path: sanitize(file.webkitRelativePath || file.name) }))
}

/**
 * Uploads a Java source tree and returns the abstraction graph.
 *
 * The backend keys each part on its *filename*, which it treats as a path
 * relative to a scratch directory, so the project-relative path is passed
 * through as the part filename. It rejects absolute paths and anything with
 * non-normal components, so paths are sanitised on the way out.
 *
 * The server parses the whole tree as a unit: a single unparseable file fails
 * the entire request with 400 and a message naming that file.
 */
export async function fetchGraph(
  files: UploadFile[],
  signal?: AbortSignal,
): Promise<RawGraph> {
  const body = new FormData()
  for (const { file, path } of files) {
    body.append('files', file, path)
  }

  const response = await fetch(`${API_URL}/graph`, {
    method: 'POST',
    body,
    signal,
  })

  if (!response.ok) {
    throw new GraphRequestError(
      response.status,
      (await response.text().catch(() => '')).trim(),
    )
  }

  return (await response.json()) as RawGraph
}

function sanitize(path: string): string {
  return path
    .replace(/^[/\\]+/, '')
    .split(/[/\\]+/)
    .filter((part) => part && part !== '.' && part !== '..')
    .join('/')
}
