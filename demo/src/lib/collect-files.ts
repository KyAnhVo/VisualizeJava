import { isJavaFile, type UploadFile } from '@/api/client'

/**
 * Walks a drop payload into a flat list of `.java` files with their
 * project-relative paths.
 *
 * Dropped directories arrive as `FileSystemEntry` handles rather than `File`s,
 * and the resulting `File` objects have no `webkitRelativePath` — so the path
 * has to be threaded through the traversal by hand.
 */
export async function collectDroppedFiles(
  items: DataTransferItemList,
): Promise<UploadFile[]> {
  const roots: FileSystemEntry[] = []
  for (const item of items) {
    const entry = item.webkitGetAsEntry?.()
    if (entry) roots.push(entry)
  }

  const collected: UploadFile[] = []
  await Promise.all(roots.map((entry) => walk(entry, '', collected)))
  collected.sort((a, b) => a.path.localeCompare(b.path))
  return collected
}

async function walk(
  entry: FileSystemEntry,
  prefix: string,
  out: UploadFile[],
): Promise<void> {
  const path = prefix ? `${prefix}/${entry.name}` : entry.name

  if (entry.isFile) {
    if (!isJavaFile(entry.name)) return
    const file = await readFile(entry as FileSystemFileEntry)
    out.push({ file, path })
    return
  }

  if (!entry.isDirectory) return
  const children = await readDirectory(entry as FileSystemDirectoryEntry)
  await Promise.all(children.map((child) => walk(child, path, out)))
}

function readFile(entry: FileSystemFileEntry): Promise<File> {
  return new Promise((resolve, reject) => {
    entry.file(resolve, reject)
  })
}

/**
 * `readEntries` returns at most ~100 entries per call and signals the end of the
 * directory with an empty batch, so it has to be drained in a loop.
 */
async function readDirectory(
  entry: FileSystemDirectoryEntry,
): Promise<FileSystemEntry[]> {
  const reader = entry.createReader()
  const all: FileSystemEntry[] = []

  for (;;) {
    const batch = await new Promise<FileSystemEntry[]>((resolve, reject) => {
      reader.readEntries(resolve, reject)
    })
    if (!batch.length) return all
    all.push(...batch)
  }
}
