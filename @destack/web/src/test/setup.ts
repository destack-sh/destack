import { fileURLToPath } from "node:url";
import { CompilerPool, DependencyTree, Workspace } from "@destack/build";
import { PackageCopy, testStore } from "@destack/build/test";
import { present } from "@destack/schema";

/** This package's directory, whose closure the served applications run on. */
const PACKAGE = fileURLToPath(new URL("../..", import.meta.url));

/** The view package the served applications render with. */
const VIEW = fileURLToPath(new URL("../../node_modules/@destack/view", import.meta.url));

/** The website fixture, whose closure the website's build tests run on. */
const WEBSITE = new URL("../../tests/fixture/website/source/", import.meta.url);

/** Store and install the builds the served applications run on, once per run. */
export default async function setup(): Promise<void> {
    // store every unit of view's closure and of this package's
    await using store = await testStore();
    await using compilers = await CompilerPool.open(store);
    const viewUnits = await Workspace.open(VIEW, compilers);
    const webUnits = await Workspace.open(PACKAGE, compilers);
    const view = present((await viewUnits.storeUnits({})).get(viewUnits.root.name), "view");
    const web = present((await webUnits.storeUnits({})).get(webUnits.root.name), "web");

    // store the units of the website fixture's closure
    const website = await PackageCopy.open(WEBSITE, compilers);
    await website[Symbol.asyncDispose]();

    // install both in the workspace's tree, as the served applications take them
    const dependencies = { [view.package.name]: view, [web.package.name]: web };
    await DependencyTree.install(dependencies, PACKAGE, store);
}
