import { followRuns } from "./test/run.ts";
import { EVERYTHING } from "./test/query.ts";

followRuns({
    "everything through a replica": {
        queries: EVERYTHING,
        seed: 17,
        bursts: 20,
        reconnect: 6,
        isReplicated: true,
    },
});
