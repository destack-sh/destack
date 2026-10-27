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
    "filters moved to a tree in place": {
        queries: FILTERS,
        seed: 29,
        bursts: 16,
        reconnect: 5,
        reshape: TREE,
        isReshapedInPlace: true,
    },
    "a tree moved to filters in place": {
        queries: TREE,
        seed: 31,
        bursts: 16,
        reconnect: 5,
        reshape: FILTERS,
        isReshapedInPlace: true,
    },
});
