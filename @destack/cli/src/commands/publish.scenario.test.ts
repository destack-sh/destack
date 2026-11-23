import { Observation } from "@destack/object";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { type JsonValue, present, schema } from "@destack/schema";
import type { Installation } from "@destack/stack/object";
import { expect, onTestFinished, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { TerminalFixture, type TerminalInput } from "../test/terminal.ts";

/** The checkout Example publishes the notebooks package from. */
const NOTEBOOKS = fileURLToPath(new URL("../../tests/fixture/notebooks/", import.meta.url));

/** The workspace packages the notebooks package requires, which a standalone checkout links. */
const REQUIRED = ["access", "account", "locale", "object", "package", "schema"] as const;

/** The release a standalone checkout requires each Destack package at. */
const REQUIRED_RELEASE = "2026.9.0";

/** The committer of a test's checkout. */
const GIT_IDENTITY = ["-c", "user.name=Test", "-c", "user.email=test@example.test"] as const;

/** The first release of the fixture clock's month, which the registry assigns a new package. */
const NEXT_RELEASE = "2026.10.0";

/** The notebooks package as its checkout declares it. */
const PACKAGE = {
    id: "package-01996ab0-0000-7000-8000-0000000000d4",
    name: "@example/notebooks",
    version: "2026.11.0",
} as const;

/** A printed installation's identifier. */
const Created = schema.looseObject({ id: schema.identifier("installation") });

/** A printed release's version and image. */
const Published = schema.looseObject({
    releases: schema.array(
        schema.looseObject({
            package: schema.looseObject({ version: schema.string() }),
            image: schema.string(),
        }),
    ),
});

/** A printed installation revision's build. */
const Revised = schema.looseObject({ buildId: schema.identifier("build") });

/** A printed build's source, image and state. */
const Built = schema.looseObject({
    /** What the build compiled. */
    source: schema.jsonObject(),
    /** The digest of the stored build's image. */
    image: schema.string(),
    /** How the build ended. */
    state: schema.string(),
});

/** Run a command line in the owner's home, printing JSON. */
async function json(
    terminal: TerminalFixture,
    words: readonly string[],
    input: TerminalInput = {},
): Promise<JsonValue> {
    // run it, refusing a failure
    const printed = await terminal.printed(["--space", "home", ...words, "--json"], input);
    expect({ code: printed.code, error: printed.error }).toEqual({ code: 0, error: "" });

    // read the last line, past any progress it showed
    const last = present(printed.output.trim().split("\n").at(-1), `${words.join(" ")}'s output`);

    return schema.json().parse(JSON.parse(last));
}

/** Read the build an installation's targeted revision runs, through the command line. */
async function build(terminal: TerminalFixture, target: Installation) {
    // read the revision, then its build
    const revisionId = present(target.targetRevisionId, `the revision of ${target.name}`);
    const revision = Revised.parse(await json(terminal, ["installation-revision", revisionId]));
    const built = Built.parse(await json(terminal, ["stack:build", revision.buildId]));

    return { source: built.source, image: built.image, state: built.state };
}

/** Lay out the notebooks package as a standalone working tree, committed or with a change. */
async function notebooksCheckout(state: "committed" | "uncommitted"): Promise<string> {
    // copy the package into a working tree of its own, requiring Destack's packages as releases
    const { files, process } = testHost();
    const root = await files.realPath(await files.makeTemporaryDirectory("destack-notebooks-"));
    onTestFinished(() => files.remove(root, { recursive: true }));
    const directory = join(root, "notebooks");
    await files.mkdir(join(directory, "src"), { recursive: true });
    for (const file of ["bunfig.toml", "destack.json", "tsconfig.json", "src/package.ts"]) {
        await files.copyFile(join(NOTEBOOKS, file), join(directory, file));
    }
    const required = Object.fromEntries(
        REQUIRED.map((name) => [`@destack/${name}`, REQUIRED_RELEASE]),
    );
    const manifest = {
        name: PACKAGE.name,
        type: "module",
        exports: { "./package": "./src/package.ts" },
        dependencies: required,
    };
    await files.writeTextFile(join(directory, "package.json"), JSON.stringify(manifest));
    const lock = {
        lockfileVersion: 2,
        workspaces: { "": { name: "@example/workspace" }, notebooks: { name: PACKAGE.name } },
        packages: {},
    };
    await files.writeTextFile(join(root, "bun.lock"), JSON.stringify(lock));
    await files.writeTextFile(join(root, ".gitignore"), "node_modules/\n");

    // link each required package to the workspace's, as an install would
    await files.mkdir(join(directory, "node_modules", "@destack"), { recursive: true });
    for (const name of REQUIRED) {
        const installed = dirname(
            fileURLToPath(import.meta.resolve(`@destack/${name}/package.json`)),
        );
        await files.symlink(installed, join(directory, "node_modules", "@destack", name));
    }

    // commit the files, leaving a change uncommitted when asked
    const git = (...command: readonly string[]) =>
        process.spawn(["git", "-C", root, ...GIT_IDENTITY, ...command]).exited;
    const codes = [
        await git("init", "--quiet"),
        await git("add", "--all"),
        await git("commit", "--quiet", "-m", "notebooks"),
    ];
    expect(codes).toEqual([0, 0, 0]);
    if (state === "uncommitted") {
        await files.writeTextFile(join(directory, "src/change.ts"), "export const change = 1;\n");
    }

    return directory;
}

/** Read an installation's name, what it follows and its readiness, without its change time. */
function readiness(target: Installation) {
    const { name, followsType, conditions } = target;
    const { lastTransitionAt: _changed, ...ready } = present(conditions["Ready"], "a Ready");

    return { name, followsType, ready };
}

/** Report whether an installation is current with its latest change. */
function isReady(target: Installation): boolean {
    return Observation.progress(target) === "current";
}

test("follow a published release's tag, then pin the installation to the release", async () => {
    // create the example organization as the owner
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" });
    const words = ["organization", "create", "--name", "Example", "--handle", "example"];
    const organized = await terminal.printed([...words, "--residency-id", "eu"]);

    // publish a committed checkout under latest, allowing the stack to publish, then tag its version stable
    const directory = await notebooksCheckout("committed");
    const published = Published.parse(
        await json(terminal, ["publish", directory, "--tag", "latest"], { stdin: "yes\n" }),
    );
    const released = present(published.releases.at(-1), "the published checkout");
    const specifier = `${PACKAGE.name}@${released.package.version}`;
    const tagged = await terminal.printed(["tag", specifier, "stable"]);

    // install the notebooks following stable
    const following = Created.parse(
        await json(terminal, ["install", `${PACKAGE.name}@stable`, "--name", "notebooks"]),
    );
    const converged = await terminal.installed(following.id, isReady);

    // pin it to the release instead, until it runs it at the new generation
    const { image } = released;
    const { version } = released.package;
    await json(terminal, ["install", specifier, "--name", "notebooks"]);
    const settled = await terminal.installed(
        following.id,
        (target) => isReady(target) && target.followsType === "release",
    );

    // run the one release, following stable and then pinned, on the build publishing made
    const builds = [await build(terminal, converged), await build(terminal, settled)];
    const followed = present(builds[0], "the followed build").source;
    const machine = terminal.computer.machine.id;
    const source = {
        kind: "checkout",
        checkout: followed["checkout"],
        directory: "notebooks",
        machine,
        commit: followed["commit"],
    };
    const ready = {
        status: "true",
        observedGeneration: 1,
        reason: "Deployed",
        message: "0 workloads deployed",
    };
    expect({
        organized: [organized.code, organized.error],
        tagged: [tagged.code, tagged.output, tagged.error],
        version,
        installations: [readiness(converged), readiness(settled)],
        builds,
    }).toEqual({
        organized: [0, ""],
        tagged: [0, `Tagged  release '${PACKAGE.name}@${version}' stable\n`, ""],
        version: NEXT_RELEASE,
        installations: [
            { name: "notebooks", followsType: "tag", ready },
            {
                name: "notebooks",
                followsType: "release",
                ready: { ...ready, observedGeneration: 2 },
            },
        ],
        builds: [
            { source, image, state: "succeeded" },
            { source, image, state: "succeeded" },
        ],
    });
});

test("publish a committed checkout at the registry's next release, whatever its package file says, and refuse uncommitted changes", async () => {
    // create the example organization as the owner
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" });
    const words = ["organization", "create", "--name", "Example", "--handle", "example"];
    await terminal.printed([...words, "--residency-id", "eu"]);

    // publish a committed checkout, allowing the stack to publish with --yes, then one with uncommitted changes
    const committed = await notebooksCheckout("committed");
    const published = Published.parse(await json(terminal, ["publish", committed, "--yes"]));
    const { version } = present(published.releases.at(-1), "the published checkout").package;
    const uncommitted = await notebooksCheckout("uncommitted");
    const refused = await terminal.printed(["--space", "home", "publish", uncommitted, "--yes"]);

    // release at the month's first, and refuse the uncommitted changes
    expect({
        released: [PACKAGE.version, version],
        refused: [
            refused.code,
            refused.output,
            refused.error.replace(/build-[0-9a-f-]{36}/u, "<build>"),
        ],
    }).toEqual({
        released: ["2026.11.0", NEXT_RELEASE],
        refused: [
            1,
            "Preparing\nBuilding\nStoring\nBuild succeeded\n",
            "error  cannot publish build '<build>': it has uncommitted changes, commit them first\n",
        ],
    });
});
