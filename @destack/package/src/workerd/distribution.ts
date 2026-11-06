/// <reference path="./supplied.d.ts" />
import { DistributionManifest } from "../definition/distribution.ts";

/** The distribution a Worker's deploy supplies to its bundle as `destack:distribution`. */
export const WorkerdDistribution = {
    /** Read the supplied manifest, at run time, as only the deploy's bundle has it. */
    async read(): Promise<DistributionManifest> {
        const supplied: { readonly default: unknown } = await import("destack:distribution");

        return DistributionManifest.parse(supplied.default);
    },
};
