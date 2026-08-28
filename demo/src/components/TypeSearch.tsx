import { Search } from 'lucide-react'
import { useMemo, useRef, useState, type KeyboardEvent } from 'react'

import type { GraphModel, TypeKind, TypeModel } from '@/model/build'

const KIND_ACCENT: Record<TypeKind, string> = {
  class: 'var(--kind-class)',
  interface: 'var(--kind-interface)',
  enum: 'var(--kind-enum)',
  annotation: 'var(--kind-annotation)',
}

const LIMIT = 8

/**
 * Find-and-jump.
 *
 * Past a couple of hundred types the canvas cannot label everything legibly at
 * a zoom that fits, so panning around hunting for a name stops working. This is
 * the way in: type a fragment, hit enter, land on the node.
 */
export function TypeSearch({
  model,
  onPick,
}: {
  model: GraphModel
  onPick: (typeKey: string) => void
}) {
  const [query, setQuery] = useState('')
  const [active, setActive] = useState(0)
  const input = useRef<HTMLInputElement>(null)

  const results = useMemo(() => {
    const needle = query.trim().toLowerCase()
    if (!needle) return []

    const hits: { type: TypeModel; rank: number }[] = []
    for (const type of model.types.values()) {
      if (!type.key.toLowerCase().includes(needle)) continue
      const simple = type.simpleName.toLowerCase()
      // A match on the type's own name beats one that only hits the package.
      const rank = simple.startsWith(needle) ? 0 : simple.includes(needle) ? 1 : 2
      hits.push({ type, rank })
    }

    return hits
      .sort(
        (a, b) =>
          a.rank - b.rank ||
          a.type.displayName.length - b.type.displayName.length ||
          a.type.key.localeCompare(b.type.key),
      )
      .slice(0, LIMIT)
      .map((hit) => hit.type)
  }, [model, query])

  const pick = (type: TypeModel | undefined) => {
    if (!type) return
    onPick(type.key)
    setQuery('')
    setActive(0)
    input.current?.blur()
  }

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    // Escape clears the box rather than reaching the canvas' reset handler.
    if (event.key === 'Escape') {
      event.stopPropagation()
      if (query) setQuery('')
      else input.current?.blur()
      return
    }
    if (event.key === 'Enter') return pick(results[active])
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      setActive((i) => Math.min(i + 1, results.length - 1))
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault()
      setActive((i) => Math.max(i - 1, 0))
    }
  }

  return (
    <div className="absolute left-1/2 top-3 z-20 w-80 -translate-x-1/2">
      <div className="flex items-center gap-2 rounded-lg border border-border bg-card/90 px-2.5 py-1.5 backdrop-blur focus-within:border-ring">
        <Search className="size-3.5 shrink-0 text-muted-foreground" />
        <input
          ref={input}
          value={query}
          onChange={(event) => {
            setQuery(event.target.value)
            setActive(0)
          }}
          onKeyDown={onKeyDown}
          placeholder={`find one of ${model.types.size} types…`}
          className="w-full bg-transparent font-mono text-[11px] outline-none placeholder:text-muted-foreground"
        />
      </div>

      {results.length > 0 && (
        <ul className="mt-1 overflow-hidden rounded-lg border border-border bg-card/95 backdrop-blur">
          {results.map((type, index) => (
            <li key={type.key}>
              <button
                type="button"
                onMouseEnter={() => setActive(index)}
                onClick={() => pick(type)}
                className={`flex w-full items-center gap-2 px-2.5 py-1.5 text-left font-mono ${
                  index === active ? 'bg-muted' : ''
                }`}
              >
                <span
                  className="size-1.5 shrink-0 rounded-[1px]"
                  style={{ background: KIND_ACCENT[type.kind] }}
                />
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-[11px]">
                    {type.displayName}
                  </span>
                  <span className="block truncate text-[9px] text-muted-foreground">
                    {type.packageName}
                  </span>
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
