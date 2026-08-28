# Run Proof visual thesis — paper-cut operations desk

## Direction and rationale

Run Proof is an evidence ledger, not another glowing observability dashboard. Its world is a small paper-cut operations desk: scheduled runs are clipped tickets moving across a ruled ledger, while a brass proof stamp marks evidence that can be carried away. Layered paper edges make sequence and provenance visible without weakening the seriousness of the record. The interface is deliberately warm, tactile, and compact; decoration only explains the movement from schedule to start to finish.

## Palette

- `ink #18231f` / muted ink `#52625b`: archival type on paper (contrast 14.6:1 and 5.4:1 on sheet).
- `desk #173b35` / deep desk `#0d2925`: the green cutting mat surrounding the ledger.
- `sheet #fff9e9` / raised paper `#fffdf5` / rule `#d8d1bd`: warm receipt stock and physical layers.
- `signal #e66a3c` with dark text `#2a160f`: the vermilion proof stamp and primary action.
- `success #176b4d`, `warning #8a5700`, `danger #a43831`: ink-like semantic marks, always paired with labels and shapes.
- Dark treatment uses `#102c28` as the page, `#193b35` as raised paper, `#f7f1df` text, `#b8c8c0` muted text, and the same semantic hues shifted lighter. The default is the light ledger treatment; `prefers-color-scheme` supplies the dark cutting-desk treatment.

## Typography and spacing

Headings use the self-hosted slab face **Bitter** (OFL, subset WOFF2) to evoke stamped filing labels. Interface and tabular data use **Atkinson Hyperlegible Next** (OFL, subset WOFF2) for unambiguous run IDs and status counts. Type steps: 14, 16, 20, 25, 32, and a fluid 44–64 px display. Body is never below 16 px. Numbers use tabular figures. Spacing follows an 8 px base with 4 px optical adjustments; the content measure is 1200 px and prose stays under 70 characters.

## Shape, depth, and interaction grammar

Paper panels have restrained 2–4 px corner cuts rather than generic rounded cards. A left binding rail and horizontal rules establish the ledger. Status is a stamped word plus a distinct glyph. Primary actions depress by 2 px like a rubber stamp; expanding a run unfolds immediately below its row. Focus is a 3 px amber outline with 3 px offset. All touch targets are at least 44 px.

Desktop keeps the summary, filters, and evidence table together. At 390 px, ornamental hero copy shortens, the illustration moves behind the masthead, summary slips become a two-column grid, and each evidence row becomes a labeled paper ticket; no horizontal table scrolling is required.

## Motion policy

New ledger rows enter with a 180 ms downward paper-settle using only opacity and transform. Filter changes cross-fade in 150 ms. The proof stamp lands once after a successful ingest/export (220 ms, no bounce loop). Under `prefers-reduced-motion: reduce`, all transforms and smooth scrolling are removed and state changes are instant opacity swaps.

## Asset plan and provenance

- `assets/src/run-proof-diorama.png`: original generated hero still showing a paper scheduler, worker path, and stamped receipt on a green cutting mat. Generated 2026-08-28 with the factory Azure image deployment, then reviewed and optimized to responsive WebP/AVIF. It is explanatory artwork, not evidence or a product screenshot.
- Hand-authored SVG marks in the UI use simple ticket, clock, check, and contradiction geometry; they are original to this product.
- The 192 px and 512 px PWA icons are raster exports of the hand-authored receipt/check favicon; no new generated source or third-party asset was introduced.
- Fonts are redistributed under the SIL Open Font License and self-hosted; no runtime font or script CDN.

### Prompt sheet

Subject: an isometric miniature operations desk where a cream paper schedule ticket travels through a small mechanical worker gate and becomes a receipt with an abstract blank circular stamp; visible separate paper layers show intent, start, and completion. World/materials: hand-cut card stock, fiber texture, brass clips, ruled ledger, dark forest-green cutting mat. Light/lens: soft upper-left studio light, shallow but readable depth, orthographic three-quarter view. Palette words: forest ink, warm ivory, vermilion, muted brass. Composition: wide landscape with quiet negative space on the left, machinery on the right. Negative list: no people, no hands, no text, no letters, no numbers, no logos, no watermark, no screens, no gradients, no plastic 3D, no neon, no photoreal corporate office.

Generated imagery is disclosed in the product footer. Original output is licensed with the repository under MIT; model provenance is the factory Azure image deployment (`factory-image`).
