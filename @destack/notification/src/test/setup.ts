import { TestRun } from "@destack/db/test/run";

/** Open the run of notification's tests. */
export default TestRun.setup(new URL(import.meta.url));
