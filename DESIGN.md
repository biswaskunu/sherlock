---
name: sherlock
description: Local-only field instrument for sampling, tracing, and correlating system slowdowns.
colors:
  ground: "#000000"
  ink: "#ffffff"
  hairline: "rgba(255, 255, 255, 0.34)"
  faint-rule: "rgba(255, 255, 255, 0.16)"
  inverted-faint: "rgba(0, 0, 0, 0.25)"
typography:
  display:
    fontFamily: 'ui-monospace, "Cascadia Mono", "JetBrains Mono", Menlo, Consolas, monospace'
    fontSize: "clamp(2.4rem, 6vw, 4.2rem)"
    fontWeight: 800
    lineHeight: 0.95
    letterSpacing: "-0.02em"
  headline:
    fontFamily: 'ui-monospace, Menlo, Consolas, monospace'
    fontSize: "15px"
    fontWeight: 700
    letterSpacing: "0.2em"
  body:
    fontFamily: 'ui-monospace, Menlo, Consolas, monospace'
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: 'ui-monospace, Menlo, Consolas, monospace'
    fontSize: "11px"
    fontWeight: 400
    letterSpacing: "0.16em"
rounded:
  sharp: "0"
spacing:
  gutter: "28px"
  gutter-mobile: "16px"
  field-gap: "22px"
components:
  tab-idle:
    backgroundColor: "{colors.ground}"
    textColor: "{colors.ink}"
    rounded: "{rounded.sharp}"
    padding: "12px 28px"
  tab-active:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.ground}"
    rounded: "{rounded.sharp}"
    padding: "12px 28px"
  button-row:
    backgroundColor: "{colors.ground}"
    textColor: "{colors.ink}"
    rounded: "{rounded.sharp}"
    padding: "10px 20px"
  button-row-hover:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.ground}"
    rounded: "{rounded.sharp}"
    padding: "10px 20px"
  status-live:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.ground}"
    rounded: "{rounded.sharp}"
    padding: "8px 12px"
  inverted-panel:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.ground}"
    rounded: "{rounded.sharp}"
    padding: "14px 16px 16px"
---

# Design System: sherlock

## Overview

**Creative North Star: "The Datamatics Field"**

The dashboard is a black-and-white field instrument, not a card console. Every pixel is measurement: barcode bar-fields drawn from live ticks, hairline graticules, sine-trace charts, and dense tabular ledgers in a single monospace voice. Emphasis comes from exactly one device, full-frame inversion, never from color, shadow, or decoration.

The world fuses a data-sublime black/white field (dense columns, inversion as selection) with bench-instrument discipline: a cross-check sweep reading order across the stat strip, labeled LAMP blocks instead of colored dots, spike lifecycle states (absent / forming / vivid / dispersed), and numbered first-run pedagogy. The previous soft dark-card dashboard is an explicit anti-reference.

**Key Characteristics:**
- Two values only: ground black, ink white; grey lives solely in hairline structure, never in reading text.
- One monospace voice with tabular numerals at every size, from monumental readouts to 11px labels.
- Rules divide, boxes never nest: full-bleed fields separated by 1px rules, zero radius everywhere.
- Values snap, never tween; the correlate panel inverts instead of animating.

## Colors

Pure black ground with white ink; structure in translucent white hairlines.

### Neutral
- **Field Black** (#000000): every ground, canvas, and input surface.
- **Signal White** (#ffffff): all reading text, traces, bars, and the inverted panel ground.
- **Graticule Hair** (rgba(255, 255, 255, 0.34)): section dividers, chart grids, control borders.
- **Faint Rule** (rgba(255, 255, 255, 0.16)): row dividers inside ledgers, never behind text.
- **Inverted Faint** (rgba(0, 0, 0, 0.25)): row dividers inside the inverted spike panel.

### Named Rules (optional, powerful)
**The Two-Value Rule.** No grey text, no accent color, no gradient. If it must be read, it is pure white on pure black (or the exact inverse). Grey exists only as 1px structure.
**The Inversion Rule.** Selection and alarm are expressed by flipping black and white (active tab, live lamp, correlate panel). Nothing else on the page may invert, so inversion always means state.

## Typography

**Display Font:** system monospace stack (ui-monospace, Cascadia Mono, JetBrains Mono, Menlo, Consolas) — the instrument voice at every size.
**Body Font:** same stack. There is no second voice.
**Label/Mono Font:** same stack; labels are distinguished by size, tracking, and case, never by family.

**Character:** Engineered and tabular. Numerals never shift width (font-variant-numeric: tabular-nums everywhere); headings are short uppercase blocks with wide tracking; body copy stays sentence case.

### Hierarchy
- **Display** (800, clamp(2.4rem, 6vw, 4.2rem), 0.95): SHERLOCK masthead only.
- **Headline** (700, 15px, 0.2em tracking, uppercase): field titles (CH1, LEDGER, SCAN, CORRELATION).
- **Title** (800, clamp(2rem, 4.5vw, 3.2rem), 1.0): live stat readouts (CPU %, MEM MB).
- **Body** (400, 14px, 1.5): ledger rows, notes, empty states.
- **Label** (400, 11px, 0.16–0.18em tracking, uppercase): column headers, addresses, captions, footer.

### Named Rules (optional)
**The Label-Is-Short Rule.** Uppercase with wide tracking is reserved for labels under ~40 characters. Sentence case carries everything longer (notes, empty states, footer).

## Layout

Full-bleed vertical field: masthead → cross-check stat strip (3 columns) → data barcode strip → tab menu row → trace fields → ledgers → footer. Sections are separated by 1px full-width rules; content gutters are 28px desktop, 16px mobile. Trace canvases and ledgers span gutter-to-gutter with no nesting. The stat strip collapses to one column under 720px; ledgers scroll horizontally instead of reflowing. More space sits above each field title (22px) than below it (10px).

## Elevation & Depth

No shadows anywhere, by doctrine. Depth is conveyed by inversion (live lamp, active tab, correlate panel) and by rule weight (1px ink dividers vs. faint row rules). Surfaces never lift; state changes snap.

### Named Rules (optional)
**The Flat Field Rule.** box-shadow is banned. If two regions need separation, use a rule or invert one of them.

## Shapes

Sharp instrument geometry: zero radius on everything, 1px rectilinear borders, square lamps. Ledger CPU cells carry inline bar meters (120px desktop, 72px mobile) drawn as bordered tracks with solid ink fills. Chart canvases are plain bordered fields; the data barcode strip is a canvas of live CPU bars, hidden until the first frame arrives.

## Components

### Tabs (menu rows)
Bordered row of uppercase buttons divided by hairline rules; the active cell inverts to white ground / black ink. Hover underlines; focus shows a 2px white outline offset 3px.

### Buttons (control rows)
Borderless cells inside a 1px bordered control bar (range inputs, SCAN, CORRELATE PEAK). Hover inverts the cell. Disabled dims to 45% opacity.

### Status lamp
Bordered block with a square lamp glyph and two-line label. Three labeled states, never color-only: LIVE (inverted block), STALLED — half-filled lamp (carrier open, no frames 10s+), OFFLINE (hollow lamp, retry copy with endpoint). role=status with aria-live.

### Stat readouts
Three cross-check cells (CH1 CPU, CH2 MEM, FRAME tick) in fixed sweep order with monumental tabular numerals and small unit labels.

### Ledger tables
Captioned tabular ledgers with uppercase hairline headers, faint row rules, right-aligned numerals, and inline bar meters scaled to the frame peak. No zebra striping, no row hover chrome.

### Inverted spike panel
White-ground panel (black ink, black rules) holding the spike verdict, exact-vs-nearest frame note, and the culprit ledger. Appears only through correlation; its inversion is the page's single emphasis event.

### Range inputs
Native datetime-local fields restyled to transparent black with mono ink (color-scheme: dark), divided by hairline rules inside the control bar.

## Do's and Don'ts

### Do:
- **Do** keep every reading in pure white on black (21:1) with tabular numerals.
- **Do** label every state in words (LIVE / STALLED / OFFLINE / ABSENT / VIVID), never by glyph or shade alone.
- **Do** hide data canvases until frames exist; empty states teach the 4-step ritual (run, spike, scan, invert).
- **Do** mark nearest-frame correlations as approximate (±5s tolerance).

### Don't:
- **Don't** introduce color, grey body text, gradients, shadows, or border-radius.
- **Don't** nest cards or add icon-plus-text tiles; rules divide the field.
- **Don't** tween values or strobe the field; Chart.js animation stays off and motion respects prefers-reduced-motion.
- **Don't** use monospace as decoration; it voices data, controls, and instrument labels only.
