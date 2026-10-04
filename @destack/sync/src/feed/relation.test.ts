import { followRuns, seededRuns } from "./test/run.ts";
import { CHAINS, COMPUTED, LOOKUPS, RELATED, ROLLUPS } from "./test/query.ts";

followRuns(
    seededRuns({
        chains: CHAINS,
        extras: COMPUTED,
        related: RELATED,
        lookups: LOOKUPS,
        rollups: ROLLUPS,
    }),
);
