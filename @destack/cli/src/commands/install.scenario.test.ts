import { PUBLISHER } from "@destack/machine/computer/test";
import { schema, Version } from "@destack/schema";
import { expect, test } from "@destack/test";
import { jsonLines, TerminalFixture, type TerminalInput } from "../test/terminal.ts";

/** The home's world: Acme with its notebooks published. */
const ACME = { organizations: [PUBLISHER], published: ["notebooks"] };

/** A printed installation. */
const Installed = schema.object({
    id: schema.identifier("installation"),
    name: schema.string(),
    package: schema.string(),
    version: schema.string(),
    tag: schema.string().nullable(),
});

/** The printed statuses of an installation, as far as the scenario reads them. */
const Statuses = schema.array(
    schema.looseObject({
        follows: schema.string(),
        installation: schema.looseObject({ servedRevisionId: schema.string().nullable() }),
    }),
);

/** The printed activities of the space's history, as far as the scenario reads them. */
const Activities = schema.array(
    schema.looseObject({
        parentType: schema.string(),
        parentId: schema.string(),
        fields: schema.array(schema.string()),
        changes: schema.number(),
    }),
);

/** The printed calls of the space's call log, by method. */
const Calls = schema.array(
    schema.looseObject({ data: schema.looseObject({ method: schema.string() }) }),
);

/** Read the identifier and name of the last installation a command printed. */
function identify(printed: { readonly output: string }) {
    const { id, name } = Installed.parse(jsonLines(printed.output).at(-1));

    return [id, name];
}

test("install a published app, returning once the space serves its revision", async () => {
    // open the owner's home, the notebooks published by their acme organization
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" }, [], ACME);
    const run = (words: readonly string[]) => terminal.printed(["--space", "home", ...words]);

    // install the latest release in the owner's home, which returns once the home serves it
    const installing = await run(["install", "@acme/notebooks", "--json"]);
    const lines = jsonLines(installing.output);
    const installed = Installed.parse(lines.at(-1));
    const statuses = Statuses.parse(lines.slice(0, -1));
    const served = await terminal.installed(installed.id, () => true);

    // watch the notebooks while creating one, then stop the watch
    const watching = terminal.start(["--space", "home", "notebooks", "--watch", "--json"]);
    await watching.waitLines(1);
    const created = await run(["notebook", "create", "--title", "Plans", "--json"]);
    await watching.waitLines(2);
    watching.interrupt();
    const watched = await watching.printed;
    const calls = Calls.parse(JSON.parse((await run(["calls", "--json"])).output));
    const history = await run(["object:activities", "--json"]);
    const activities = Activities.parse(JSON.parse(history.output));
    const notebook = schema.looseObject({ id: schema.string() }).parse(JSON.parse(created.output));

    // install once the home serves the revision, show no notebooks then the created one, and log its call and its history
    expect({
        installing: [installing.code, installing.error],
        followed: [
            installed.tag,
            statuses.at(-1)?.follows,
            statuses.at(-1)?.installation.servedRevisionId,
        ],
        served: served.servedRevisionId,
        installed,
        version: Version.release(installed.version),
        watched: [watched.code, jsonLines(watched.output), watched.error],
        calls: calls
            .map((call) => call.data.method)
            .filter((method) => method.split(".")[0] === "notebook"),
        activities: activities.map(({ parentType, parentId, fields, changes }) => ({
            parentType,
            parentId,
            fields,
            changes,
        })),
    }).toEqual({
        installing: [0, ""],
        // follow the latest tag, showing the installation until the space serves its revision
        followed: ["latest", "tag latest", served.targetRevisionId],
        served: served.targetRevisionId,
        installed: {
            id: installed.id,
            tag: "latest",
            name: "notebooks",
            package: "@acme/notebooks",
            version: installed.version,
        },
        // install the release as the run built it, unreleased
        version: Version.development,
        watched: [130, [[], [JSON.parse(created.output)]], ""],
        calls: ["notebook.create"],
        // record the creation as one activity on the notebook, with the fields it wrote
        activities: [
            {
                parentType: "notebook",
                parentId: notebook.id,
                fields: ["ownerPackageId", "ownerType", "ownerScope", "ownerId", "title"],
                changes: 1,
            },
        ],
    });
});

test("keep a package's one installation on a second install, and refuse another name for it", async () => {
    // install acme's published notebooks in the owner's home
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" }, [], ACME);
    const run = (words: readonly string[], input: TerminalInput = {}) =>
        terminal.printed(["--space", "home", ...words], input);
    const first = await run(["install", "@acme/notebooks", "--json"]);

    // install them again, under no name and under another
    const again = await run(["install", "@acme/notebooks", "--json"]);
    const renamed = await run(["install", "@acme/notebooks", "--name", "journal"]);
    expect({
        installed: [first.code, again.code, identify(again)],
        renamed: [renamed.code, renamed.error],
    }).toEqual({
        installed: [0, 0, identify(first)],
        renamed: [
            1,
            "error  installation 'notebooks' already exists: it is the package's one installation in the space, so leave --name out\n",
        ],
    });
});

test("uninstall a package's installation once confirmed, and refuse one the space lacks", async () => {
    // install the acme notebooks in the owner's home
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" }, [], ACME);
    const run = (words: readonly string[]) => terminal.printed(["--space", "home", ...words]);
    const names = async () =>
        schema
            .array(schema.looseObject({ name: schema.string() }))
            .parse(JSON.parse((await run(["installations", "--json"])).output))
            .map(({ name }) => name);
    await run(["install", "@acme/notebooks", "--json"]);
    const before = await names();

    // refuse its delete and its uninstall unconfirmed, uninstall it once confirmed, then by its name again
    const deleting = await run(["installation", "notebooks", "delete"]);
    const unconfirmed = await run(["uninstall", "@acme/notebooks"]);
    const uninstalling = await run(["uninstall", "@acme/notebooks", "--yes"]);
    const after = await names();
    const again = await run(["uninstall", "notebooks", "--yes"]);
    const { home } = terminal.computer.world.owner;
    expect({
        deleting: [deleting.code, deleting.error],
        unconfirmed: [unconfirmed.code, unconfirmed.error],
        uninstalling: [uninstalling.code, uninstalling.output, uninstalling.error],
        removed: before.filter((name) => !after.includes(name)),
        again: [again.code, again.error],
    }).toEqual({
        deleting: [
            1,
            "error  cannot run 'delete' on 'notebooks': it cannot be undone, so confirm it with --yes\n",
        ],
        unconfirmed: [
            1,
            "error  cannot uninstall 'notebooks': it deletes the installation's resources, so confirm it with --yes\n",
        ],
        uninstalling: [0, "Uninstalled  installation 'notebooks'\n", ""],
        removed: ["notebooks"],
        again: [1, `error  installation 'notebooks' of '${home}' not found\n`],
    });
});
