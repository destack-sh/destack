import { join } from "node:path";
import { expect, refusal, test } from "@destack/test";
import { Workspace } from "./workspace.ts";
import { testSimulation } from "../test/world.ts";

test("refuse a frozen install of a package with dependencies and no lockfile, passing one without any", async () => {
    // write one package with a dependency and one without, neither locked
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await files.realPath(await files.makeTemporaryDirectory("destack-workspace-"));
    const locked = join(root, "notes");
    const bare = join(root, "bare");
    await files.mkdir(locked, { recursive: true });
    await files.mkdir(bare, { recursive: true });
    await files.writeTextFile(
        join(locked, "package.json"),
        JSON.stringify({ name: "@ada/notes", dependencies: { "@destack/object": ">=2026.11.0" } }),
    );
    await files.writeTextFile(join(bare, "package.json"), JSON.stringify({ name: "@ada/bare" }));

    // install both frozen
    const signal = new AbortController().signal;
    const options = { isFrozen: true, signal };
    expect({
        locked: await refusal(Workspace.install(locked, environment, options)),
        bare: await Workspace.install(bare, environment, options),
    }).toEqual({
        locked: [
            "BAD_REQUEST",
            "DEPENDENCY_INVALID",
            "dependencies of '@ada/notes' are invalid: it has no lockfile, so run bun install and commit bun.lock",
        ],
        bare: undefined,
    });
});
