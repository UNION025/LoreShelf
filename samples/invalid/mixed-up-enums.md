---
lorespec: "0.1"
id: "sample-all-object-types"
date: "2026-01-01"
source: "other"
topic: "A synthetic sample that uses every LoreSpec v0.1 object type once"
tags: [sample, test-fixture]
classification:
  type: technical
  secondary_type: strategy
  domains: [testing]
  value: low
trails: [sample-trail]
---

## Session Arc

### Started

A synthetic session used only to test that a fully valid file is accepted.

### Pivots

- The sample was widened from one object type to all eight so that every field check is exercised.

### Ended

The sample contains one object of each type, linked together.

## Artifacts

### A1 — Sample document

- **Title:** Sample document
- **Type:** doc
- **Status:** provisional
- **Version note:** First version.
- **Summary:** A placeholder artifact that exists to be linked from other objects.
- **Links:** D1

## Decisions

### D1 — Use a synthetic sample

- **Decision:** Tests use a synthetic Lore instead of real conversation data.
- **Issue:** Which data should the validator tests read?
- **Positions:**
  - Read real Lore from a local folder.
  - Use a small synthetic file kept in the repository.
- **Arguments:**
  - Real Lore may be private and may not exist on another machine.
  - A synthetic file is available everywhere the repository is cloned.
- **Warrant:** Tests must run from a fresh clone.
- **Qualifier:** in this case
- **Status:** settled
- **Links:** A1, I1

## Insights

### I1 — Fresh clones must pass tests

- **Insight:** A test that depends on files outside the repository fails on any machine that lacks them.
- **Source:** analysis
- **Domain:** testing
- **Confidence:** established
- **Links:** D1, P1

## Patterns

### P1 — Synthetic fixture

- **Name:** Synthetic fixture
- **Description:** Keep a small invented input in the repository for tests that would otherwise need private or machine-specific data.
- **Steps or components:**
  - Write the smallest input that exercises the behavior.
  - Commit it next to the tests.
  - Read it by a path relative to the repository.
- **Scope:** universal
- **Origin:** research
- **Links:** I1

## Open Questions

### Q1 — How large should the fixture be?

- **Question:** Should the sample cover every field or only the required ones?
- **Context:** A fuller sample catches more mistakes but is longer to maintain.
- **Partial answers:**
  - Covering every object type once is a reasonable middle ground.
- **Blocks:** Nothing.
- **Links:** D1

## References

### R1 — LoreSpec

- **Name:** LoreSpec
- **Type:** repo
- **Relevance:** The specification this sample conforms to.
- **URL:** https://github.com/lorespec-org/lorespec
- **Links:** D1

## Next Steps

### N1 — Add more samples

- **Action:** Add samples for edge cases as they are found.
- **Why:** Each bug found by hand should become a fixture.
- **Depends on:** D1
- **Urgency:** someday
- **Links:** Q1

## Solutions

### S1 — Test fails on a fresh clone

- **Problem:** A test read a file that exists only on one machine.
- **Fix:** Replace the file with a synthetic sample inside the repository.
- **Why it works:** Every clone contains the sample, so the test no longer depends on local state.
- **Caveats:** The sample must be kept in step with the specification.
- **Links:** I1

## Connections

- I1 —[led_to]→ D1
- D1 —[led_to]→ A1
- I1 —[informed_by]→ R1
- P1 —[related_to]→ I1
- Q1 —[led_to]→ N1
- N1 —[depends_on]→ D1
- S1 —[related_to]→ I1

## Trail Updates

- **New trail: Sample Trail** — created by this synthetic sample.
