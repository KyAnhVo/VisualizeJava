# Name Resolution
The goal for this module is to accept an AST (from Parser module) and
resolve all the names that are related to this project.

# Method

## Idea

Before the algorithm, we first understand how Java does its name resolution. Here is the order:
- Type declared in the same file
- Single type import
- Type declared in the same package
- Wildcard import
- java.lang default import

So the idea is that we have a scope table such that:
- Scope[name] = (package, typename)
And we set up in the reverse direction:
- Put java.lang classes in first
- Put wildcard imports in
- Put types declared in the same package in,
- Put single type imports in
- Put types declared in the same file in.
For which put in means fill in if not there, or override if there.
For example, take the following code:
```
package com.current; // where we have a different file of same package with Character class
import com.example.util.*; // have a Character class
import com.example.npc.Character;
public class Character {...}
```
We consider Scope[Character]:
1. Character
2. java.lang.Character
3. com.example.util.Character
4. com.example.npc.Character;
5. com.example.npc.Character; (current file)
### NOTE 1: 
Although, for package building, we would sweep over the files in the same package
and verify that no 2 top level classes of a class have the same name so 4 and 5 are
not necessarily clashing / or raise error right away.
### NOTE 2: 
Also, we would not consider java.lang since it is over our scope and we do not want
to draw abstraction/dependency edges to and from java.lang and anything not inside project
file.

## Algorithm (pseudocode)

Here is the pseudocode for the algo algorithm:

### Phase 1: Flatten
For each file:
- Recursively flatten the types
- Put the file with all types at 1st level (so file[type].members is empty) into its
  corresponding package: Map<Package Name -> Vector<Files>>

### Phase 2: Name Resolution
Name resolution is done via BFS-ing the file.

#### Logic:
The logic is as such:
If `A` is an outer type of `B`, B depends on A.
If `B extends A` or `B implements A`, B depends on A.
From the 2 definitions, we construct a "depends on" graph, where `B depends on A` induces an edge `(A, B)`
And the graph has no cycle for a compiled java program.
Thus we solve type by BFS, which here is equivalent to topological ordered type stuff.

#### Algorithm:
Construct a FIFO queue TypeQueue
Append all top level types of all the AST's in.
while the queue is not empty:
- dequeue the type, and:
  - if any of the type's depends-on (here, `extends` or `implements` edge) is not resolved:
    - enqueue it back to the list.
  - else:
    - resolve its generics, its parents, its members
    - enqueue its inner types
The termination condition is either:
- the queue is empty, hence success
- there is no new resolved type after 1 round, failure.


