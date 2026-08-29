# Demo reports

## 2026-08-28

### Problems with current design:

- Show every node is not useful:
  - A lot of nodes in a real project doesn't contribute in any inheritance/implement hierarchy.
  - These nodes will over-populate the screen
- Probably WASM this project, since backend doesn't contribute to any state saving / caching stuffs

### Solution:

- Divide by package, thus not many nodes on a graph at once
- Yes, wasm.
