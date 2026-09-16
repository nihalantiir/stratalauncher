fabric.svg, forge.svg, neoforge.svg, and quilt.svg (the plain outline marks,
shown for a loader row when it's not the selected one) are sourced verbatim
from Modrinth's own open-source monorepo:

https://github.com/modrinth/code/tree/main/packages/assets/icons/tags/loaders

That `packages/assets` package is licensed under the GNU GPLv3
(see https://github.com/modrinth/code/blob/main/packages/assets/LICENSE).
Strata itself is GPL-3.0 licensed (see the repo's own LICENSE file), so
this dependency is already license-compatible; no separate action needed.

vanilla.svg is an original, hand-drawn generic "open box" outline (the same
isometric-cube glyph shape used across many unrelated MIT-licensed icon
sets), not derived from Mojang/Minecraft artwork, so there's no licensing concern.

full/fabric.png, full/forge.png, full/neoforge.png, full/quilt.png, and
full/lwjgl.png (the real colored marks, shown once that loader is selected,
or in the Components panel) are each project's own official GitHub
organization avatar, fetched directly from avatars.githubusercontent.com:
FabricMC, MinecraftForge, neoforged, QuiltMC, and LWJGL respectively. These
are each project's own trademark/branding, not covered by Strata's own
GPL-3.0 license (a copyright license, not a trademark grant); used here
purely to identify compatibility (the same nominative use every other
Minecraft launcher's loader picker makes of these same marks), not a claim
of endorsement.
