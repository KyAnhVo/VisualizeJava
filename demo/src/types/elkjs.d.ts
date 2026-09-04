/**
 * Vite's `?worker` suffix has no types for a dependency path, and elkjs's own
 * `elk-worker.d.ts` is just `export type Worker = Worker`, which is
 * self-referential and unusable. Declare the worker-entry form we import.
 */
declare module 'elkjs/lib/elk-worker.min.js?worker' {
  const ElkEngineWorker: new () => Worker
  export default ElkEngineWorker
}
