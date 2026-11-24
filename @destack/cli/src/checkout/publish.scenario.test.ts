import { Observation } from "@destack/object";
import { join } from "node:path";
import { type JsonValue, present, schema } from "@destack/schema";
import type { Installation } from "@destack/stack/object";
import { expect, test } from "@destack/test";
import { TerminalFixture, type TerminalInput } from "../test/terminal.ts";
import { PUBLISHER } from "@destack/machine/computer/test";

/** The world of every scenario: the example organization publishing the starter template. */
const PUBLISHED = { organizations: [PUBLISHER], published: ["starter"] };

/** The first release of the fixture clock's month, which the registry assigns a new package. */
const NEXT_RELEASE = "2026.10.0";

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

/** Make the owner's notebooks package from the starter template, with a change when asked. */
async function notebooks(terminal: TerminalFixture, state: "committed" | "uncommitted") {
    // make it under the owner's handle in a new directory
    const { files } = terminal.native.environment;
    const name = `@${terminal.computer.world.owner.handle}/notebooks-${state}`;
    const directory = join(terminal.native.home, `notebooks-${state}`);
    const template = ["--template", "@example/starter"];
    const made = await terminal.printed(["new", name, directory, ...template]);
    expect([made.code, made.error]).toEqual([0, ""]);

    // leave a change uncommitted when asked
    if (state === "uncommitted") {
        await files.writeTextFile(join(directory, "src/change.ts"), "export const change = 1;\n");
    }

    return { name, directory };
}

/** Read an installation's name, its version and its readiness, without its change time. */
function readiness(target: Installation) {
    const { name, versionType, conditions } = target;
    const { lastTransitionAt: _changed, ...ready } = present(conditions["Ready"], "a Ready");

    return { name, versionType, ready };
}

/** Report whether an installation is current with its latest change. */
function isReady(target: Installation): boolean {
    return Observation.progress(target) === "current";
}

test("follow a published release's tag, then pin the installation to the release", async () => {
    // open the owner's terminal in the world publishing the starter template
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" }, [], PUBLISHED);

    // publish a committed checkout under latest, allowing the stack to publish, then tag its version stable
    const { name, directory } = await notebooks(terminal, "committed");
    const published = Published.parse(
        await json(terminal, ["publish", directory, "--tag", "latest"], { stdin: "yes\n" }),
    );
    const released = present(published.releases.at(-1), "the published checkout");
    const specifier = `${name}@${released.package.version}`;
    const tagged = await terminal.printed(["tag", specifier, "stable"]);

    // install the notebooks following stable
    const following = Created.parse(
        await json(terminal, ["install", `${name}@stable`, "--name", "notebooks"]),
    );
    const converged = await terminal.installed(following.id, isReady);

    // pin it to the release instead, until it runs it at the new generation
    const { image } = released;
    const { version } = released.package;
    await json(terminal, ["install", specifier, "--name", "notebooks"]);
    const settled = await terminal.installed(
        following.id,
        (target) => isReady(target) && target.versionType === "release",
    );

    // run the one release, following stable and then pinned, on a build of the release
    const builds = [await build(terminal, converged), await build(terminal, settled)];
    const { files } = terminal.native.environment;
    const declared = schema
        .looseObject({ id: schema.string() })
        .parse(JSON.parse(await files.readTextFile(join(directory, "destack.json"))));
    const source = { kind: "release", package: declared.id, version, image };
    const ready = {
        status: "true",
        observedGeneration: 1,
        reason: "Deployed",
        message: "0 workloads deployed",
    };
    expect({
        tagged: [tagged.code, tagged.output, tagged.error],
        version,
        installations: [readiness(converged), readiness(settled)],
        builds,
    }).toEqual({
        tagged: [0, `Tagged  release '${name}@${version}' stable\n`, ""],
        version: NEXT_RELEASE,
        installations: [
            { name: "notebooks", versionType: "tag", ready },
            {
                name: "notebooks",
                versionType: "release",
                ready: { ...ready, observedGeneration: 2 },
            },
        ],
        builds: [
            { source, image, state: "succeeded" },
            { source, image, state: "succeeded" },
        ],
    });
});

test("publish a committed checkout at the registry's next release, and refuse uncommitted changes before pushing", async () => {
    // open the owner's terminal in the world publishing the starter template
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" }, [], PUBLISHED);

    // publish a committed checkout, allowing the stack to publish with --yes, then one with uncommitted changes
    const committed = await notebooks(terminal, "committed");
    const published = Published.parse(
        await json(terminal, ["publish", committed.directory, "--yes"]),
    );
    const { version } = present(published.releases.at(-1), "the published checkout").package;
    const uncommitted = await notebooks(terminal, "uncommitted");
    const refused = await terminal.printed([
        "--space",
        "home",
        "publish",
        uncommitted.directory,
        "--yes",
    ]);

    // release at the month's first, and refuse the uncommitted changes before pushing or building anything
    expect({
        released: version,
        refused: [refused.code, refused.output, refused.error],
    }).toEqual({
        released: NEXT_RELEASE,
        refused: [
            1,
            "",
            `error  cannot push the HEAD of '${uncommitted.directory}': it has uncommitted changes, so commit them first\n`,
        ],
    });
});
