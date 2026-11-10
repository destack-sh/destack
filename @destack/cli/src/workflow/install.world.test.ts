import { fileURLToPath } from "node:url";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { jsonLines, TerminalFixture } from "../test/terminal.ts";

/** The checkout Acme publishes the notebooks package from. */
const NOTEBOOKS = fileURLToPath(new URL("../../tests/fixture/notebooks/", import.meta.url));

/** The notebooks package, as its checkout declares it: a dev build of an uncommitted tree. */
const VERSION = "2026.11.0-dev.1";

/** How long the scenario takes at most: a build of the checkout, a publish and a rollout. */
const SCENARIO_MILLISECONDS = 120_000;

/** A printed installation. */
const Installed = schema.object({
    id: schema.identifier("installation"),
    name: schema.string(),
    package: schema.string(),
    version: schema.string(),
    selection: schema.object({ kind: schema.literal("tag"), tag: schema.string() }),
});

/** The printed statuses of an installation's rollout, as far as the scenario reads them. */
const Statuses = schema.array(
    schema.looseObject({
        rollout: schema.looseObject({ servedRevisionId: schema.string().nullable() }),
    }),
);

/** The printed calls of the space's call log, by method. */
const Calls = schema.array(
    schema.looseObject({ data: schema.looseObject({ method: schema.string() }) }),
);

test(
    "install a published app, waiting until the space serves it, then write its objects while a watch shows them",
    async () => {
        // publish the notebooks as the owner's acme organization
        const terminal = await TerminalFixture.open({ NO_COLOR: "1" });
        const run = (words: readonly string[]) => terminal.printed(["--space", "home", ...words]);
        const organization = ["--name", "Acme", "--handle", "acme", "--residency-id", "eu"];
        const organized = await terminal.printed(["organization", "create", ...organization]);
        const published = await terminal.printed(
            ["--space", "home", "publish", NOTEBOOKS, "--tag", "latest", "--json"],
            { stdin: "yes\n" },
        );

        // install the latest release in the owner's home, which returns once the home serves it
        const installing = await run(["install", "@acme/notebooks", "--json"]);
        const lines = jsonLines(installing.output);
        const installed = Installed.parse(lines.at(-1));
        const statuses = Statuses.parse(lines.slice(0, -1));
        const served = await terminal.rollout(installed.id, () => true);

        // watch the notebooks while creating one, then stop the watch
        const watching = terminal.start(["--space", "home", "notebooks", "--watch", "--json"]);
        await watching.waitLines(1);
        const created = await run(["notebook", "create", "--title", "Plans", "--json"]);
        await watching.waitLines(2);
        watching.interrupt();
        const watched = await watching.printed;
        const calls = Calls.parse(JSON.parse((await run(["calls", "--json"])).output));

        // install once the home serves the revision, show no notebooks then the created one, and log its call
        expect({
            organized: organized.code,
            published: [published.code, published.error],
            installing: [installing.code, installing.error],
            followed: [
                installed.selection,
                statuses.at(-1)?.rollout.servedRevisionId === served.revisionId,
            ],
            installed,
            watched: [watched.code, jsonLines(watched.output), watched.error],
            calls: calls
                .map((call) => call.data.method)
                .filter((method) => method.split(".")[0] === "notebook"),
        }).toEqual({
            organized: 0,
            published: [0, ""],
            installing: [0, ""],
            // follow the latest tag, showing its rollout until the space serves its revision
            followed: [{ kind: "tag", tag: "latest" }, true],
            installed: {
                id: installed.id,
                selection: { kind: "tag", tag: "latest" },
                name: "notebooks",
                package: "@acme/notebooks",
                version: VERSION,
            },
            watched: [130, [[], [JSON.parse(created.output)]], ""],
            calls: ["notebook.create"],
        });
    },
    SCENARIO_MILLISECONDS,
);
