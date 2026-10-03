# LoreShelf

**A local-first library for your AI conversation knowledge.**

LoreShelf is a desktop application for storing, searching, exploring, and continuing knowledge captured in [LoreSpec](https://github.com/lorespec-org/lorespec) (`LORE.md`).

LoreSpec solves the problem of turning AI conversations into durable, structured knowledge.

LoreShelf focuses on what comes next:

**What happens when you have hundreds or thousands of `LORE.md` files?**

LoreShelf organizes them into a local personal library, indexes their knowledge objects, and makes old conversations discoverable through full-text and semantic search.

The original `LORE.md` files remain portable and independent of LoreShelf.

## Why LoreShelf?

You may remember discussing something with an AI months ago without remembering the exact conversation, wording, or even which AI you used.

LoreShelf is designed around that problem.

Search approximately:

> "I think I talked about dynamic IPs before..."

Find the relevant Lore, inspect what was learned, and use it as context for another conversation — even with a different AI provider.

## Built on LoreSpec

LoreShelf uses LoreSpec as its knowledge format rather than defining another incompatible AI conversation format.

LoreSpec defines the structure and semantics of `LORE.md`.

LoreShelf provides the library around it:

- Local storage and organization
- Full-text search
- Semantic search
- Knowledge-object browsing
- Trail exploration
- Conversation lineage
- Import and export
- Local statistics and discovery

LoreShelf is an independent project and is not an official LoreSpec application.

## Local First

Your AI conversation knowledge should not depend on a server continuing to exist.

LoreShelf is designed to work locally and offline.

```text id="8qxig2"
LORE.md files
     │
     ▼
  LoreShelf
     │
     ├── Local database / index
     ├── Full-text search
     ├── Semantic search
     └── Lineage & metadata
```

The database is an index and management layer, not a proprietary replacement for your Lore.

Your knowledge remains portable.

## Trails vs. Lineage

LoreSpec already defines **Trails**, which connect related knowledge across sessions.

LoreShelf additionally explores **Lineage**.

They answer different questions:

**Trail:** "How are these ideas related?"

**Lineage:** "Was this conversation actually continued from that previous context?"

Two conversations may discuss nearly identical topics without sharing lineage.

A child belongs to a lineage only when it was created by continuing from an inherited Lore context.

```text id="e05i0d"
Lore A
  │
  │ context inherited
  ▼
Lore B
  │
  │ context inherited
  ▼
Lore C
```

Semantic similarity does not create lineage.
