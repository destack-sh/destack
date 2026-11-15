import { TestRun } from "@destack/db/test/run";

/** Open the run of message's tests. */
export default TestRun.setup(new URL(import.meta.url));
