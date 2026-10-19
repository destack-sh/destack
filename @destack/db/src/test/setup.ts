import { TestRun } from "./run.ts";

/** Write db's test templates once per run. */
export default TestRun.setup(new URL("./templates.ts", import.meta.url));
