import { followRuns, seededRuns } from "./test/run.ts";
import { FILTERS, MEMORY_AUDIENCE, PATHS, TALLIES, TREE, WINDOWS } from "./test/query.ts";

/** The query sets of row shapes: filters, windows, trees, tallies and paths. */
const SHAPES = { filters: FILTERS, windows: WINDOWS, tree: TREE, tallies: TALLIES, paths: PATHS };

followRuns({
    ...seededRuns(SHAPES),
    ...Object.fromEntries(
        Object.entries(SHAPES).map(([name, queries]) => [
            `${name} decided in memory`,
            { queries, seed: 37, bursts: 16, reconnect: 5, audience: MEMORY_AUDIENCE },
        ]),
    ),
    "windows moved to filters": {
        queries: WINDOWS,
        seed: 29,
        bursts: 16,
        reconnect: 5,
        reshape: FILTERS,
    },
});
