import { join } from "node:path";
import { Digest, present, schema } from "@destack/schema";
import { expect, test } from "@destack/test";

import { NpmInstallation } from "./npm.ts";
import { convertHere, copy, moduleFailure, writeStore } from "./test/store.ts";
import { DependencyTree } from "./tree.ts";
import { LocalPackageStore } from "../store/local.ts";

/** The toolchain identity the units key by. */
const CONTEXT = await Digest.json({ test: "toolchain" });

/** The default export of a module. */
const DefaultExport = schema.looseObject({ default: schema.unknown() });

test("place npm units with the peers the packages beside them provide, failing a missing peer once it loads", async () => {
    // install a renderer whose peer is a core package, and a list depending on a store whose peer it is
    const { root, locked } = await writeStore({
        renderer: {
            manifest: { main: "index.js", peerDependencies: { core: "1.0.0" } },
            files: { "index.js": 'module.exports = "rendered with " + require("core");\n' },
        },
        core: {
            manifest: { main: "index.js" },
            files: { "index.js": 'module.exports = "core";\n' },
        },
        store: {
            manifest: { main: "index.js", dependencies: { lists: "1.0.0" } },
            files: { "index.js": 'module.exports = require("lists");\n' },
            links: ["lists"],
        },
        lists: {
            manifest: { main: "index.js", peerDependencies: { store: "1.0.0" } },
            files: { "index.js": 'module.exports = typeof require("store");\n' },
        },
    });
    await using store = await LocalPackageStore.temporary();
    const installation = new NpmInstallation(locked, store, CONTEXT);
    const build = async (name: string) =>
        await installation.store(await installation.open(copy(root, name)), convertHere);
    const [renderer, core, lists] = [
        await build("renderer"),
        await build("core"),
        await build("store"),
    ];

    // place the renderer beside its core, and the store, whose list requires it as its peer
    const tree = await DependencyTree.materialize(
        store,
        [renderer, core, lists],
        join(root, "tree"),
    );
    const exported = async (name: string) =>
        DefaultExport.parse(await import(join(present(tree.directories[name], name), "index.js")))
            .default;

    // render with the provided core, read the store through its list, and fail a renderer whose tree lacks the core once it loads
    const lacking = await DependencyTree.materialize(store, [renderer], join(root, "lacking"));
    const lacked = join(present(lacking.directories["renderer"], "renderer"), "index.js");
    expect({ renderer: await exported("renderer"), store: await exported("store") }).toEqual({
        renderer: "rendered with core",
        store: "object",
    });
    expect(await moduleFailure(import(lacked))).toEqual([
        "MODULE_NOT_FOUND",
        `Cannot find module 'core'\nRequire stack:\n- ${lacked}`,
    ]);
});
