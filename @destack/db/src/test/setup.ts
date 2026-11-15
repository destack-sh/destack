import { TestRun } from "./run.ts";

/** Open the run of db's tests. */
export default TestRun.setup(new URL(import.meta.url));
