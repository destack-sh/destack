import { followAtScale, followRuns, seededRuns } from "./test/run.ts";
import { EVERYTHING, FILTERS, TREE, WITHIN } from "./test/query.ts";

followRuns({
    ...seededRuns({ within: WITHIN, everything: EVERYTHING }),
    "everything through a replica": {
        queries: EVERYTHING,
        seed: 17,
        bursts: 20,
        reconnect: 6,
        isReplicated: true,
    },
    "filters moved to a tree": {
        queries: FILTERS,
        seed: 23,
        bursts: 16,
        reconnect: 5,
        reshape: TREE,
    },
});

followAtScale("everything at scale", {
    queries: EVERYTHING,
    seed: 31,
    bursts: 6,
    reconnect: 2,
    seedRows: 3000,
    isReplicated: true,
});
