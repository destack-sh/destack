import { followRuns, seededRuns } from "./test/run.ts";
import { EVERYTHING, WITHIN } from "./test/query.ts";

followRuns(seededRuns({ within: WITHIN, everything: EVERYTHING }));
