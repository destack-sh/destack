import { join } from "node:path";
import type { FileSystem } from "@destack/environment";
import { DISTRIBUTION_MANIFEST, DistributionManifest } from "../definition/distribution.ts";

/** The distribution a Bun machine ships in a directory, its manifest beside its layer. */
export const BunDistribution = {
    /** Read the manifest in a distribution's directory. */
    async read(directory: string, files: FileSystem): Promise<DistributionManifest> {
        const text = await files.readTextFile(join(directory, DISTRIBUTION_MANIFEST));

        return DistributionManifest.parse(JSON.parse(text));
    },
};
