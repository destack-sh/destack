import { Observation } from "@destack/object";
import { fileURLToPath } from "node:url";
import { type JsonValue, present, schema, Version } from "@destack/schema";
import type { InstallationRollout } from "@destack/stack/object";
import { expect, test } from "@destack/test";
import { TerminalFixture, type TerminalInput } from "../test/terminal.ts";

/** The checkout Acme publishes the notebooks package from. */
const NOTEBOOKS = fileURLToPath(new URL("../../tests/fixture/notebooks/", import.meta.url));

/** The first release of the fixture clock's month, which the registry assigns a new package. */
const NEXT_RELEASE = "2026.10.0";

/** The notebooks package as its checkout declares it. */
const PACKAGE = {
    id: "package-01996ab0-0000-7000-8000-0000000000d4",
    name: "@acme/notebooks",
    version: "2026.11.0",
} as const;

/** A printed installation's identifier. */
const Created = schema.looseObject({ id: schema.identifier("installation") });

/** A printed release's version and lock. */
const Published = schema.looseObject({
    releases: schema.array(
        schema.looseObject({
            package: schema.looseObject({ version: schema.string() }),
            lock: schema.string(),
        }),
    ),
});

/** A printed installation revision's build. */
const Revised = schema.looseObject({ buildId: schema.identifier("build") });

/** A printed build's source, lock and state. */
const Built = schema.looseObject({
    /** What the build compiled. */
    source: schema.jsonObject(),
    /** The digest of the stored build's lock. */
    lock: schema.string(),
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
async function build(terminal: TerminalFixture, target: InstallationRollout) {
    // read the revision, then its build
    const revisionId = present(
        target.targetRevisionId,
        `the revision of ${target.installation.name}`,
    );
    const revision = Revised.parse(await json(terminal, ["installation-revision", revisionId]));
    const built = Built.parse(await json(terminal, ["stack:build", revision.buildId]));

    return { source: built.source, lock: built.lock, state: built.state };
}

/** Read an installation's name, what it follows and its readiness, without its change time. */
function readiness(target: InstallationRollout) {
    const { name, followsType, conditions } = target.installation;
    const { lastTransitionAt: _changed, ...ready } = present(conditions["Ready"], "a Ready");

    return { name, followsType, ready };
}

/** Report whether an installation is current with its latest change. */
function isReady(target: InstallationRollout): boolean {
    return Observation.progress(target.installation) === "current";
}

test("follow a published release's tag, then pin the installation to the release", async () => {
    // create the acme organization as the owner
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" });
    const words = ["organization", "create", "--name", "Acme", "--handle", "acme"];
    const organized = await terminal.printed([...words, "--residency-id", "eu"]);

    // publish the checkout under latest, allowing the stack to publish, then tag its version stable
    const published = Published.parse(
        await json(terminal, ["publish", NOTEBOOKS, "--tag", "latest"], { stdin: "yes\n" }),
    );
    const released = present(published.releases.at(-1), "the published checkout");
    const specifier = `${PACKAGE.name}@${released.package.version}`;
    const tagged = await terminal.printed(["tag", specifier, "stable"]);

    // install the notebooks following stable
    const following = Created.parse(
        await json(terminal, ["install", `${PACKAGE.name}@stable`, "--name", "notebooks"]),
    );
    const converged = await terminal.rollout(following.id, isReady);

    // pin it to the release instead, until it runs it at the new generation
    const { lock } = released;
    const { version } = released.package;
    await json(terminal, ["install", specifier, "--name", "notebooks"]);
    const settled = await terminal.rollout(
        following.id,
        (target) => isReady(target) && target.installation.followsType === "release",
    );

    // run the one release, following stable and then pinned, on the build publishing made
    const builds = [await build(terminal, converged), await build(terminal, settled)];
    const checkout = present(builds[0], "the followed build").source["checkout"];
    const directory = "@destack/cli/tests/fixture/notebooks";
    const machine = terminal.computer.machine.id;
    const source = { kind: "checkout", checkout, directory, machine };
    const ready = {
        status: "true",
        observedGeneration: 1,
        reason: "Deployed",
        message: "0 workloads deployed",
    };
    expect({
        organized: [organized.code, organized.error],
        tagged: [tagged.code, tagged.output, tagged.error],
        version: Version.release(version),
        installations: [readiness(converged), readiness(settled)],
        builds,
    }).toEqual({
        organized: [0, ""],
        tagged: [0, `Tagged ${PACKAGE.name}@${version} stable.\n`, ""],
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
            { source, lock, state: "succeeded" },
            { source, lock, state: "succeeded" },
        ],
    });
});

test("publish an uncommitted checkout as a dev build of the registry's next release, whatever its package file says", async () => {
    // create the acme organization as the owner
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" });
    const words = ["organization", "create", "--name", "Acme", "--handle", "acme"];
    await terminal.printed([...words, "--residency-id", "eu"]);

    // publish the checkout twice, allowing the stack to publish
    const publish = async () => {
        const published = Published.parse(
            await json(terminal, ["publish", NOTEBOOKS], { stdin: "yes\n" }),
        );

        return present(published.releases.at(-1), "the published checkout").package.version;
    };
    const first = await publish();
    const second = await publish();

    // lead a dev build of unchanged, uncommitted source to the registry's next release both times
    expect([
        PACKAGE.version,
        Version.release(first),
        Version.isPrerelease(first),
        second === first,
    ]).toEqual(["2026.11.0", NEXT_RELEASE, true, true]);
});
