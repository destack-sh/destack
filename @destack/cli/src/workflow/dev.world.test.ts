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

/** How long the scenario takes at most: a build of the checkout and a preview. */
const SCENARIO_MILLISECONDS = 120_000;

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

test(
    "build a checkout's package in the space through each phase, and start a preview the machine runs on",
    async () => {
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
    },
    SCENARIO_MILLISECONDS,
);
