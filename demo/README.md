# DEMO

## Goal:
Create a demo for the frontend of the app, where it should demonstrate the dependency graph in java.

## Tech
React, Tailwind, Shadcn, React flow

## Style
- Depends on you, but make dark mode first class (and only dark mode).
- Make the inheritance edges very clear.

## Spec
- Only draw inheritance edge. Do not draw other edges unless one of the below conditions occur.
- For inner/outer edge, embed the inner type inside the block of the outer type
- For association member-to-type-edge:
  - Make each property/method clickable on its own
  - On member clicked, dim all other unrelated types/edges, and draw association edges that relate to this member only.
- For association type-to-member edge:
  - Make the outer box of type clickable on its own
  - on type clicked, draw association edge to members (of own type or other type) that uses it. Dim all other unrelated type/edge.
- For any click not on the member/edge, reset all dim/etc.
- Treat implement and inheritance edges as the same thing, although color code it differently and make the inheritance edge thicker and more clear than implement edges.
