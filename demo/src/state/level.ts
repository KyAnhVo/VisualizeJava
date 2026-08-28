/**
 * Which slice of the graph the canvas is showing.
 *
 * Rendering every type stopped being viable somewhere below this project's own
 * `test_target/prod`: 257 types come back as a 6791×4100 canvas that fits a
 * 1600×900 viewport at zoom 0.22, where 11px text renders under 3px. Canvas
 * size grows as √(type count), so no amount of layout work rescues a real
 * codebase. The answer is to navigate rather than to draw everything.
 */
export type Level =
  | { kind: 'packages' }
  | { kind: 'package'; name: string }
  | { kind: 'focus'; typeKey: string }
  /** The old whole-graph view, kept as an escape hatch. */
  | { kind: 'all' }

export const HOME: Level = { kind: 'packages' }

/** Identity of a level, for cache keys and effect dependencies. */
export function levelKey(level: Level): string {
  switch (level.kind) {
    case 'packages':
      return 'packages'
    case 'package':
      return `package:${level.name}`
    case 'focus':
      return `focus:${level.typeKey}`
    case 'all':
      return 'all'
  }
}
