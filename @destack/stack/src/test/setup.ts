import { TestRun } from "@destack/db/test/run";

/** Write stack's test templates once per run. */
const templates = TestRun.setup(new URL("./templates.ts", import.meta.url));

/** Write the test templates once per run. */
export default async function setup(
    project: Parameters<typeof templates>[0],
): Promise<() => Promise<void>> {
    return await templates(project);
}
