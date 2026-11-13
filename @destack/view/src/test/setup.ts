import { fileURLToPath } from "node:url";
import { Workspace, CompilerPool } from "@destack/build";
import { testStore } from "@destack/build/test";
import { Toolchain } from "@destack/check/toolchain";

/** The notebook package, whose closure the build tests compile on. */
const NOTEBOOK = fileURLToPath(new URL("../../tests/fixture/notebook/", import.meta.url));

/** Store the notebook package's dependency builds once per run. */
export default async function setup(): Promise<void> {
    await using store = await testStore();
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    await (await (await Workspace.open(NOTEBOOK, compilers)).package()).storeDependencies({});
}
