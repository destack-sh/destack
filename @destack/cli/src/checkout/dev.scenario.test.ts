import { PUBLISHER } from "@destack/machine/computer/test";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { preview } from "@destack/machine/checkout";
import { schema, Version } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { TerminalFixture } from "../test/terminal.ts";

/** The checkout the owner builds and previews the notebooks package from. */
const NOTEBOOKS = fileURLToPath(new URL("../../tests/fixture/notebooks/", import.meta.url));

/** A second, which the second preview of a checkout waits after the first. */
const SECOND = 1000;

/** The directory of the fixtures, where a test lays out a checkout that resolves the workspace. */
const FIXTURES = fileURLToPath(new URL("../../tests/fixture/", import.meta.url));

/** Write a setting of the content's language taking the given values. */
function languageSetting(values: readonly string[]): string {
    return `import { defineSetting } from "@destack/setting/declare";
import { schema } from "@destack/schema";
import { t } from "@destack/locale";

/** The language of generated content. */
export const language = defineSetting({
    name: "language",
    title: t\`Content language\`,
    description: t\`Choose the language of generated content.\`,
    schema: schema.enum(${JSON.stringify(values)}),
    default: "en",
    scope: "user",
    overrides: ["space", "installation"],
});
`;
}

/** The notebooks package as a build of its checkout that leads to no release has it. */
const PACKAGE = {
    id: "package-01996ab0-0000-7000-8000-0000000000d4",
    name: "@example/notebooks",
    version: Version.development,
} as const;

/** A printed build's state and phase. */
const Phase = schema.looseObject({ state: schema.string(), phase: schema.string().nullable() });

/** A printed build's summary. */
const Summary = schema.object({
    space: schema.string(),
    id: schema.string(),
    image: schema.string(),
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
    const rows = await database.snapshot().read(preview.table).findMany();

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
    // install example's published notebooks in the owner's home
    const published = { organizations: [PUBLISHER], published: ["notebooks"] };
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" }, [], published);
    const run = (words: readonly string[], stdin?: string) =>
        terminal.printed(["--space", "home", ...words], stdin === undefined ? {} : { stdin });
    const installed = await run(["install", "@example/notebooks", "--json"]);

    // preview the checkout, then install the release again, reading the version the installation runs
    const versions = async () => {
        const listed = await run(["installations", "--json"]);
        const rows = schema
            .array(
                schema.looseObject({
                    name: schema.string(),
                    versionType: schema.string().nullable(),
                    directory: schema.string().nullable(),
                }),
            )
            .parse(JSON.parse(listed.output));

        return rows
            .filter((row) => row.name === "notebooks")
            .map((row) => [row.versionType, row.directory]);
    };
    const released = await versions();
    const previewed = await run(["dev", NOTEBOOKS, "--json"]);
    const previewing = await versions();
    const reinstalled = await run(["install", "@example/notebooks", "--json"]);
    expect({
        codes: [installed.code, previewed.code, previewed.error, reinstalled.code],
        versions: [released, previewing, await versions()],
    }).toEqual({
        codes: [0, 0, "", 0],
        versions: [
            [["tag", null]],
            [["checkout", "@destack/cli/tests/fixture/notebooks"]],
            [["tag", null]],
        ],
    });
});

test("refuse a preview whose setting takes fewer values without a conversion, with the reason", async () => {
    // lay out the notebooks with a setting beside the fixtures, where the workspace resolves
    const { files } = testHost();
    const directory = await files.makeTemporaryDirectory("notebooks-", { directory: FIXTURES });
    onTestFinished(() => files.remove(directory, { recursive: true }));
    for (const file of ["destack.json", "bunfig.toml", "tsconfig.json", "src/package.ts"]) {
        await files.mkdir(join(directory, "src"), { recursive: true });
        await files.copyFile(join(NOTEBOOKS, file), join(directory, file));
    }
    const manifest = schema
        .looseObject({
            exports: schema.record(schema.string(), schema.string()),
            dependencies: schema.record(schema.string(), schema.string()),
        })
        .parse(JSON.parse(await files.readTextFile(join(NOTEBOOKS, "package.json"))));
    manifest.exports["./setting"] = "./src/setting.ts";
    manifest.dependencies["@destack/setting"] = "workspace:*";
    await files.writeTextFile(join(directory, "package.json"), JSON.stringify(manifest));
    const setting = join(directory, "src/setting.ts");
    await files.writeTextFile(setting, languageSetting(["en", "de", "fr"]));

    // preview it, then a second later take a value away without a conversion and preview it again
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" });
    const first = await terminal.printed(["--space", "home", "dev", directory, "--json"]);
    await terminal.computer.world.simulation.sleep(SECOND);
    await files.writeTextFile(setting, languageSetting(["en", "de"]));
    const second = await terminal.printed(["--space", "home", "dev", directory]);

    // refuse the second preview with the setting's problem, which the machine captured, at the checkout's dev build version
    const version = Version.development;
    const problem = `cannot upgrade installation 'notebooks' to '${version}': setting/language: declare a conversion for ${version}`;
    const captured = terminal.computer.world.telemetry.take().map((exception) => exception.message);
    expect({
        first: [first.code, first.error],
        second: [second.code, second.output, second.error],
        captured: [...new Set(captured)],
    }).toEqual({
        first: [0, ""],
        second: [1, "", `error  preview of '${directory}' is broken: ${problem}\n`],
        captured: [problem],
    });
});
