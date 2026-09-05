import type { UploadedFile } from '@/wasm/protocol'

/** Raised when the user hands us something that is not a directory. */
export class NotADirectoryError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'NotADirectoryError'
  }
}

/**
 * The wasm parser ignores everything else anyway; filtering here keeps the
 * progress total honest and avoids shipping unrelated blobs into the worker.
 */
function isJavaFile(path: string): boolean {
  return path.endsWith('.java')
}

/** `.git`, `.idea` and friends never contain sources we want to parse. */
function isHiddenDirectory(name: string): boolean {
  return name.startsWith('.')
}

/**
 * `readEntries` returns at most a batch at a time (100 in Chromium) and
 * signals completion with an empty batch, so it must be drained in a loop.
 * Reading it once is the classic way to silently lose files in large folders.
 */
function readAllEntries(
  reader: FileSystemDirectoryReader,
): Promise<FileSystemEntry[]> {
  return new Promise((resolve, reject) => {
    const entries: FileSystemEntry[] = []
    const readBatch = () => {
      reader.readEntries((batch) => {
        if (batch.length === 0) {
          resolve(entries)
          return
        }
        entries.push(...batch)
        readBatch()
      }, reject)
    }
    readBatch()
  })
}

function fileOf(entry: FileSystemFileEntry): Promise<File> {
  return new Promise((resolve, reject) => entry.file(resolve, reject))
}

async function walkDirectory(
  directory: FileSystemDirectoryEntry,
  prefix: string,
  collected: UploadedFile[],
): Promise<void> {
  const entries = await readAllEntries(directory.createReader())
  for (const entry of entries) {
    const path = `${prefix}/${entry.name}`
    if (entry.isDirectory) {
      if (isHiddenDirectory(entry.name)) continue
      await walkDirectory(entry as FileSystemDirectoryEntry, path, collected)
    } else if (isJavaFile(entry.name)) {
      collected.push({ path, file: await fileOf(entry as FileSystemFileEntry) })
    }
  }
}

/**
 * Collects `.java` files from a drop, rejecting anything that is not a folder.
 *
 * `webkitGetAsEntry` is the only way to see a dropped directory's contents;
 * `DataTransfer.files` flattens to the folder itself as a zero-byte entry.
 */
export async function collectFromDrop(
  transfer: DataTransfer,
): Promise<UploadedFile[]> {
  // The item list is invalidated once we await, so snapshot the entries first.
  const entries = [...transfer.items]
    .filter((item) => item.kind === 'file')
    .map((item) => item.webkitGetAsEntry())

  if (entries.length === 0) {
    throw new NotADirectoryError('Nothing was dropped. Drop a project folder.')
  }
  if (entries.some((entry) => entry === null)) {
    throw new NotADirectoryError(
      "This browser could not read the dropped item as a folder. Use the “Choose folder” button instead.",
    )
  }
  const files = entries.filter((entry) => entry!.isFile)
  if (files.length > 0) {
    throw new NotADirectoryError(
      `“${files[0]!.name}” is a file. Drop the folder that contains your Java sources.`,
    )
  }

  const collected: UploadedFile[] = []
  for (const entry of entries) {
    await walkDirectory(entry as FileSystemDirectoryEntry, entry!.name, collected)
  }
  return collected
}

/**
 * Collects `.java` files from an `<input webkitdirectory>` selection. The OS
 * dialog only offers directories, so the result is a directory by construction;
 * `webkitRelativePath` already carries the path from the chosen folder down.
 */
export function collectFromInput(fileList: FileList): UploadedFile[] {
  const collected: UploadedFile[] = []
  for (const file of fileList) {
    const path = file.webkitRelativePath || file.name
    if (!isJavaFile(path)) continue
    if (path.split('/').slice(0, -1).some(isHiddenDirectory)) continue
    collected.push({ path, file })
  }
  return collected
}

/** Root folder name, taken from the shared first segment of the paths. */
export function rootNameOf(files: UploadedFile[]): string {
  const first = files[0]?.path
  if (!first) return 'project'
  const segment = first.split('/')[0]
  return segment || 'project'
}
