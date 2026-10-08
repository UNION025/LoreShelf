---
lorespec: "0.1"
id: "sample-official-style"
date: "2026-01-02"
source: "other"
topic: "A synthetic sample written in the shape used by the official LoreSpec example"
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
A made-up session used to check that the official writing style is accepted.

### Pivots
- **Strict → Tolerant:** The check was first written for one writing style and then widened to accept the others used in practice.

### Ended
Every object type appears once, written the way the official example writes it.

## Artifacts

**A1: Sample plan**
- **Type:** plan
- **Status:** final
- **Version note:** First version.
- **Summary:** A placeholder artifact that other objects link to.
- **Links:** informed_by D1, D2; depends_on R1

## Decisions

**D1: Accept both writing styles**
- **Decision:** The check accepts headings written as `### D1 — title` and as `**D1: title**`.
- **Issue:** Real files use more than one writing style.
- **Positions:** (1) Accept only one style; (2) Accept every style seen in practice
- **Arguments:** The specification does not fix the Markdown of an object. Rejecting a style that the official example uses would reject valid files.
- **Warrant:** Because the specification, not the application, decides what is valid.
- **Qualifier:** Settled for now. Revisit if the specification fixes the markup.
- **Status:** settled
- **Links:** led_to D2, led_to A1; informed_by I1

**D2: Keep enumerated values strict**
- **Decision:** A value outside the specification's list is still refused.
- **Issue:** How strict should the check be about values?
- **Positions:** (1) Ignore values; (2) Refuse values outside the list
- **Arguments:** A borrowed value, such as another object type's status, would break any filter built on that field.
- **Warrant:** Because a filter is only useful if the values are the ones it expects.
- **Qualifier:** usually
- **Status:** provisional — revisit if real files need more values
- **Links:** informed_by I1; instance_of P1

## Insights

**I1:** Models that follow the same instruction can still write objects in different shapes, so a check that accepts only one shape rejects valid files.

**I2:** A value taken from a different object type is the commonest slip, because several types share words such as draft and provisional.

## Patterns

**P1: Be lenient about shape, strict about values**
- **Description:** Accept every layout that carries the same information, but refuse values that contradict the specification.
- **Steps:** (1) List the layouts seen in practice; (2) Parse each into the same object; (3) Check values against the specification's lists.
- **Scope:** universal — applies to any format with a loose layout and fixed vocabulary
- **Origin:** Invented during this session
- **Links:** D2 is an instance_of this pattern

## Open Questions

**OQ1: Should the qualifier be refused when it is a sentence?**
- **Question:** Is a free-text qualifier a mistake or an accepted way of writing?
- **Context:** The official example writes a sentence in this field although the table lists four values.
- **Partial answers:** It is treated as a warning for now.
- **Blocks:** Whether filters on the qualifier can be relied on.

## References

**R1: Sample specification**
- **Type:** article
- **Relevance:** Stands in for the specification this sample is written against.
- **URL:** https://example.com/spec

## Next Steps

**NS1:** Add a sample for each new writing style found
- **Why:** Each style found by hand should become a fixture.
- **Urgency:** soon
- **Links:** depends_on D1

## Solutions

**S1: A valid file was refused**
- **Problem:** A file written in the official style was rejected as having undefined objects.
- **Fix:** Recognize bold headings and inline objects as well as `###` headings.
- **Why it works:** The objects are then found, so references to them resolve.
- **Caveats:** A new style would need the same treatment.
- **Links:** I1

## Connections

- D1 —[led_to]→ D2 (the first decision made the second one necessary)
- I1 —[informed_by]→ D1, D2
- P1, I2 —[led_to]→ OQ1
- D2 —[instance_of]→ P1
- D1 —[led_to]→ NS1 —[depends_on]→ A1（a chain, with a note）
- S1 —[related_to]→ I1；R1 —[related_to]→ A1

## Trail Updates

- **New trail: Sample Trail** — created by this synthetic sample.
