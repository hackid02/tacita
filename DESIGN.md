# design rules for tacita

rules with reasons, so later changes keep the same feel. checked against the common
"ai slop" tells: violet or indigo accents, purple gradients, inter everywhere, everything
in rounded cards, a centered hero over three feature cards, scroll effects, buzzword copy.

## identity
- **mark:** a vermilion seal split by a slit that forms a T. it reads as a closed mouth
  (tacita, the roman goddess of silence) and a seal (a round locked before payout).
  files in `docs/brand/`: `tacita-mark.svg`, `tacita-logo.svg`, `tacita-logo-dark.svg`,
  `favicon.svg` (thicker slit so it survives 16px).
- **wordmark:** TACITA in marcellus, an inscriptional roman face, tracked +0.16em,
  converted to outlines so it never depends on a font loading.

## colour (five colours, each with a job)
- ink `#16130f`, soft `#5c564d`, rule `#d9d4cb`, paper `#f5f4f1`, well `#ecebe7`.
- seal `#c2361d` (dark mode `#e0533a`): roman rubrication red. only for the mark,
  the fingerprint, focus rings and the "no match" dot. never decoration.
- yes `#2c6a45` for a confirmed match. no gradients anywhere. no violet.

## type
- source serif 4 for reading, ibm plex mono for anything you might copy or compare,
  marcellus only for small capital labels. all self-hosted in `docs/fonts/` (sil ofl).
- modular scale, ratio 1.25: 14 / 17 / 21 / 26 / 33 / 41.

## layout
- editorial, left aligned, hairline rules instead of cards. asymmetric grids (7/5, 4/8).
- the real tool sits in the first screen, not a picture of it.
- facts are real numbers from the contract (16 winners, one fingerprint), never invented stats.

## motion
- one signature moment: on check, the list lines drop into the fingerprint and the
  amounts blur out. nothing else moves except button press feedback.
- ease-out `cubic-bezier(0.23, 1, 0.32, 1)`, under 300ms, never ease-in.
- buttons scale to 0.97 on press over 160ms. hover only inside
  `@media (hover: hover) and (pointer: fine)`.
- reduced motion keeps fades and drops movement and blur.

## code and copy
- one plain html file, no framework: it loads instantly and anyone can read the code they trust.
- the fingerprint logic in `docs/index.html` mirrors `cli/src/manifest.rs`. change both or
  neither, and test with the same file.
- check at 390, 768, 1024 and 1440 px, dark mode and 200% zoom. ootle addresses are about
  110 characters with no break points.
- words: lowercase, plain, no hype, no em dashes. say what it proves and what it doesn't,
  including the limits.
