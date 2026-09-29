import { Release } from "@destack/update/release";
import manifest from "../../../../@destack/desktop/package.json" with { type: "json" };

/** Version injected into every component of this distribution. */
export const version = new Release(
    process.env.DESTACK_RELEASE_VERSION ?? manifest.version,
    Release.target(),
).version;
