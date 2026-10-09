import { followRuns } from "./test/run.ts";
import { FILTERS, TREE } from "./test/query.ts";

followRuns({
    "filters moved to a tree": {
        queries: FILTERS,
        seed: 23,
        bursts: 16,
        reconnect: 5,
        reshape: TREE,
    },
});
