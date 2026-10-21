import { fileURLToPath } from "node:url";
import { Workspace } from "@destack/build";
import { testCompilers, testStore } from "@destack/build/test";

/** The notebook package, whose closure the build tests compile on. */
const NOTEBOOK = fileURLToPath(new URL("../../tests/fixture/notebook/", import.meta.url));

/** Store the notebook package's dependency builds once per run. */
export default async function setup(): Promise<void> {
    await using store = await testStore();
    await using compilers = await testCompilers();
    await (await Workspace.open(NOTEBOOK, compilers)).storeDependencies(store, {});
}
