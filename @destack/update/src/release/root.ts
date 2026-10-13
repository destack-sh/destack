import type { Channel } from "./release.ts";
import nightly from "./root/nightly.json" with { type: "json" };
import stable from "./root/stable.json" with { type: "json" };

/** The root metadata each channel's repository is trusted from, bundled with every release. */
export const CHANNEL_ROOTS = { stable, nightly } as const satisfies Record<Channel, unknown>;
