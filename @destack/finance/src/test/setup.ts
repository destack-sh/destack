import { TestRun } from "@destack/db/test/run";

/** Open the run of finance's tests. */
export default TestRun.setup(new URL(import.meta.url));
