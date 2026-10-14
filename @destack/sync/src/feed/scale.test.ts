import { followAtScale } from "./test/run.ts";
import { EVERYTHING } from "./test/query.ts";

followAtScale("everything at scale", {
    queries: EVERYTHING,
    seed: 31,
    bursts: 6,
    reconnect: 2,
    seedRows: 300,
    pageSize: 50,
    isReplicated: true,
});
