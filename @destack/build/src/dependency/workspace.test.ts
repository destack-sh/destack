import { join } from "node:path";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Workspace } from "./workspace.ts";
import { testSimulation } from "../test/world.ts";
import { testHost } from "@destack/test/host";

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
    const options = { isFrozen: true, signal, registry: undefined };
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

test("resolve a package's own, declared and builtin scopes from the machine's registry", async () => {
    // serve a registry that records what Bun asks it for and has nothing
    const asked: string[] = [];
    const server = testHost().server.http({
        hostname: "127.0.0.1",
        port: 0,
        fetch: (request) => {
            asked.push(decodeURIComponent(new URL(request.url).pathname));

            return new Response("{}", { status: 404 });
        },
    });
    onTestFinished(() => server.stop(true));

    // write a package depending on one package of each scope, declaring another publisher's
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await files.realPath(await files.makeTemporaryDirectory("destack-registry-"));
    const dependencies = {
        "@ada/widgets": "1.0.0",
        "@ben/charts": "1.0.0",
        "@destack/view": "1.0.0",
    };
    await files.writeTextFile(
        join(root, "package.json"),
        JSON.stringify({ name: "@ada/notes", dependencies }),
    );
    await files.writeTextFile(
        join(root, "destack.json"),
        JSON.stringify({
            $schema: "https://destack.app/schemas/destack.json",
            id: "package-01996ab0-0000-7000-8000-000000000001",
            role: "app",
            language: "typescript",
            scopes: ["@ben"],
        }),
    );

    // install it unfrozen with the machine's registry of the builtins' scope
    const registry = { url: `${server.url.origin}/.npm/`, scopes: ["@destack"] };
    const signal = new AbortController().signal;
    await refusal(Workspace.install(root, environment, { isFrozen: false, signal, registry }));

    // expect each scope asked of the registry, and the checkout left without a bunfig
    expect({
        asked: [...new Set(asked)].toSorted(),
        bunfig: await files.exists(join(root, "bunfig.toml")),
    }).toEqual({
        asked: ["/.npm/@ada/widgets", "/.npm/@ben/charts", "/.npm/@destack/view"],
        bunfig: false,
    });
});
