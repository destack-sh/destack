import { fileURLToPath } from "node:url";
import { Snapshot } from "@destack/db";
import { preview } from "@destack/machine/checkout";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";

/** The checkout the owner builds and previews the notebooks package from. */
const NOTEBOOKS = fileURLToPath(new URL("../../tests/fixture/notebooks/", import.meta.url));

/** The notebooks package as its checkout declares it. */
const PACKAGE = {
    id: "package-01996ab0-0000-7000-8000-0000000000d4",
    name: "@acme/notebooks",
    version: "2026.11.0",
} as const;

/** A printed build's state and phase. */
const Phase = schema.looseObject({ state: schema.string(), phase: schema.string().nullable() });

/** A printed build's summary. */
const Summary = schema.object({
    space: schema.string(),
    id: schema.string(),
    lock: schema.string(),
    package: schema.unknown(),
});

/** A printed preview. */
const Previewed = schema.looseObject({
    id: schema.string(),
    package: schema.string(),
    space: schema.string(),
});

test("build a checkout's package in the space through each phase, and start a preview the machine runs on", async () => {
    // build the checkout in the owner's home, then preview it there
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" });
    const built = await terminal.printed(["--space", "home", "build", NOTEBOOKS, "--json"]);
    const previewed = await terminal.printed(["--space", "home", "dev", NOTEBOOKS, "--json"]);

    // read the build's phases and summary, and the preview the machine keeps
    const lines = built.output.trim().split("\n");
    const summary = Summary.parse(JSON.parse(lines.at(-1) ?? ""));
    const phases = lines.slice(0, -1).map((line) => {
        const { state, phase } = Phase.parse(JSON.parse(line));

        return phase === null ? state : `${state} ${phase}`;
    });
    const started = Previewed.parse(JSON.parse(previewed.output));
    const { database } = terminal.computer.machine;
    const rows = await Snapshot.live(database).read(preview.table).findMany();

    // show the build's phases to its summary, and keep the preview running past the command line
    const { home } = terminal.computer.world.owner;
    expect({
        built: [built.code, built.error, phases, summary.space, summary.package],
        previewed: [previewed.code, previewed.error, started.space, started.package],
        previews: rows.map((row) => [row.id, row.deletedAt]),
    }).toEqual({
        built: [
            0,
            "",
            ["running preparing", "running building", "running storing", "succeeded"],
            home,
            PACKAGE,
        ],
        previewed: [0, "", home, "@destack/cli/tests/fixture/notebooks"],
        previews: [[started.id, null]],
    });
});

test("have a preview take over the package's installation, and an install take it back", async () => {
    // publish the notebooks as acme and install them in the owner's home
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" });
    const run = (words: readonly string[], stdin?: string) =>
        terminal.printed(["--space", "home", ...words], stdin === undefined ? {} : { stdin });
    const words = ["organization", "create", "--name", "Acme", "--handle", "acme"];
    await run([...words, "--residency-id", "eu"]);
    await run(["publish", NOTEBOOKS, "--json"], "yes\n");
    const installed = await run(["install", "@acme/notebooks", "--json"]);

    // preview the checkout, then install the release again, reading what the installation follows
    const follows = async () => {
        const listed = await run(["installations", "--json"]);
        const rows = schema
            .array(
                schema.looseObject({
                    name: schema.string(),
                    followsType: schema.string().nullable(),
                    directory: schema.string().nullable(),
                }),
            )
            .parse(JSON.parse(listed.output));

        return rows
            .filter((row) => row.name === "notebooks")
            .map((row) => [row.followsType, row.directory]);
    };
    const released = await follows();
    const previewed = await run(["dev", NOTEBOOKS, "--json"]);
    const previewing = await follows();
    const reinstalled = await run(["install", "@acme/notebooks", "--json"]);
    expect({
        codes: [installed.code, previewed.code, previewed.error, reinstalled.code],
        follows: [released, previewing, await follows()],
    }).toEqual({
        codes: [0, 0, "", 0],
        follows: [
            [["tag", null]],
            [["checkout", "@destack/cli/tests/fixture/notebooks"]],
            [["tag", null]],
        ],
    });
});
