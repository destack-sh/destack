import { TestRun } from "@destack/db/test/run";

/** Write account's test templates once per run. */
export default TestRun.setup(new URL("./templates.ts", import.meta.url));
