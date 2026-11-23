# design rules for tacita

rules with reasons, so later changes keep the same feel.

- **quiet over flashy.** tacita is a trust tool. no shaders, 3d or scroll effects: they slow phones down and look like every other ai-built site.
- **the real tool in the first screen.** the list checker sits right under the headline, not a mockup of it.
- **one signature motion.** on check, the list lines slide down into the fingerprint and the amounts blur out. it explains the product (amounts private, fingerprint public). nothing else moves except button press feedback.
- **fast motion, never ease-in.** ui motion stays under 300ms with an ease-out curve `cubic-bezier(0.23, 1, 0.32, 1)`. buttons scale to 0.97 on press over 160ms.
- **reduced motion means fades, not nothing.** with prefers-reduced-motion, movement and blur go and opacity fades stay.
- **hover only on a mouse.** hover styles live inside `@media (hover: hover) and (pointer: fine)`.
- **addresses are long.** ootle addresses are about 110 characters with no break points. values use `overflow-wrap: anywhere`; decorative copies truncate with an ellipsis.
- **plain html, no framework.** one file, loads instantly, and anyone can read the code they are trusting.
- **the page and the cli must agree.** the fingerprint logic in `docs/index.html` mirrors `cli/src/manifest.rs`. change both or neither, and test with the same file.
- **check at 390, 768, 1024 and 1440 px, dark mode and 200% zoom** before shipping.
- **words:** lowercase, plain, no hype, no em dashes. say what the page proves and what it doesn't.
