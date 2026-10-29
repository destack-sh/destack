import type { FileSystem } from "@destack/environment";
import { testSimulation } from "../src/test/world.ts";
import { test } from "@destack/test";
import { join } from "node:path";
import { graph } from "@destack/package";
import { PackageManifest } from "@destack/package/manifest";
import { present, schema } from "@destack/schema";
import type { PackageBuild } from "../src/build/index.ts";
import { Fixture } from "./fixture.ts";
import * as library from "./fixture/library/request.ts";

/** A card component showing a note. */
const CARD = `import { createNote, type Note } from "./note.ts";

/** The properties of a note card. */
export interface CardProperties {
    /** The note's title. */
    readonly title: string;
    /** Whether the note is done. */
    readonly isComplete?: boolean;
    /** The card's look. */
    readonly tone?: "quiet" | "loud";
    /** The lines of text shown. */
    readonly lines?: number;
    /** Run when the card opens. */
    readonly onOpen?: () => void;
}

/** Show a note as a card. */
export function NoteCard(properties: CardProperties): Note {
    return createNote(properties.title);
}
`;

/** An interaction of cards, marking one done and reading its state. */
const CARD_INTERACTION = `import type { Interaction } from "@destack/package/declare";
import { schema } from "@destack/schema";

/** A card's step: marking it done. */
const DoneStep = schema.object({ action: schema.literal("done") });

/** A card's observation: its state. */
const StateObservation = schema.object({ kind: schema.literal("state") });

/** The interaction of cards. */
export const cardInteraction: Interaction<
    schema.Infer<typeof DoneStep>,
    schema.Infer<typeof StateObservation>,
    { readonly width?: number }
> = {
    name: "card",
    step: DoneStep,
    observation: StateObservation,
    globals: schema.object({ width: schema.number().exactOptional() }),
};
`;

/** Name a moniker within its package. */
function local(moniker: string): string {
    return moniker.slice(moniker.indexOf("/") + 1);
}

/** A module declaring an example of the card and a scenario taking steps on it. */
const CARD_EXAMPLES = `import { defineExample, defineScenario } from "@destack/package/declare";
import { NoteCard } from "./card.ts";
import { cardInteraction } from "./card.interaction.ts";
import { LOUD_LOOK } from "./card.look.ts";

/** A loud card. */
export const noteCardLoud = defineExample({
    of: NoteCard,
    name: "loud",
    description: "a loud card",
    properties: { title: "Groceries", ...LOUD_LOOK, lines: 1 / -0.5 },
    render: (properties) => NoteCard({ title: "Groceries", ...properties }),
});

/** Complete a loud card. */
export const noteCardComplete = defineScenario({
    of: NoteCard,
    interaction: cardInteraction,
    name: "complete",
    description: "complete a card",
    given: { examples: [noteCardLoud], globals: { width: 320 } },
    when: [{ action: "done" }, { action: "set", properties: { tone: "quiet" } }],
    then: {
        observe: { state: { kind: "state" } },
        end: { state: "done" },
    },
});
`;

/** A module with the loud look an example spreads. */
const CARD_LOOK = `/** The look of a loud card. */
export const LOUD_LOOK = { tone: "loud" } as const;
`;

/** Add a card component with an interaction, an example and a scenario taking steps on it. */
async function writeCards(source: string, files: FileSystem): Promise<void> {
    // depend on the declaring packages
    const manifest = join(source, "package.json");
    const declared = schema.looseObject({}).parse(JSON.parse(await files.readTextFile(manifest)));
    await files.writeTextFile(
        manifest,
        JSON.stringify({
            ...declared,
            devDependencies: {
                "@destack/package": "workspace:*",
                "@destack/schema": "workspace:*",
            },
        }),
    );

    // write the card, the interaction and the module declaring the example and the scenario
    await files.writeTextFile(join(source, "src/card.ts"), CARD);
    await files.writeTextFile(join(source, "src/card.interaction.ts"), CARD_INTERACTION);
    await files.writeTextFile(join(source, "src/card.look.ts"), CARD_LOOK);
    await files.writeTextFile(join(source, "src/card.example.ts"), CARD_EXAMPLES);
}

/** Read a build's file digests by path. */
async function digests(build: PackageBuild): Promise<Map<string, string>> {
    return new Map((await build.reader.files()).map((file) => [file.path, file.digest]));
}

test.concurrent("keep each output module an edit leaves unchanged byte for byte", async ({
    expect,
}) => {
    // build the library, then reorder a function body in one module and build again
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    await using input = await Fixture.open("library", environment);
    await using before = await input.build(library.request);
    const note = join(input.source, "src/note.ts");
    const source = await files.readTextFile(note);
    await files.writeTextFile(
        note,
        source.replace("return { title, complete: false };", "return { complete: false, title };"),
    );
    await using after = await input.build(library.request);

    // leave the untouched module's output file as it was, and change the edited module's
    const [first, second] = await Promise.all([digests(before), digests(after)]);
    const module = (path: string) =>
        present(
            [...second.keys()].find((each) => each.startsWith("output/") && each.endsWith(path)),
            `the output module ${path}`,
        );
    const [index, edited] = [module("/index.js"), module("/note.js")];
    expect([
        second.get(index) === first.get(index),
        second.get(edited) === first.get(edited),
    ]).toEqual([true, false]);
});

test.concurrent("declare each test in its module's graph, and index the package symbols it calls as covered", async ({
    expect,
}) => {
    // add a suite whose test calls the library's note constructor
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    await using input = await Fixture.open("library", environment);
    const manifest = join(input.source, "package.json");
    const declared = schema.looseObject({}).parse(JSON.parse(await files.readTextFile(manifest)));
    await files.writeTextFile(
        manifest,
        JSON.stringify({ ...declared, devDependencies: { "@destack/test": "workspace:*" } }),
    );
    await files.writeTextFile(
        join(input.source, "src/note.test.ts"),
        `import { describe, test } from "@destack/test";
import { createNote } from "./note.ts";
describe("note", () => {
    test("starts incomplete", () => {
        createNote("title");
    });
});
`,
    );
    const { code, modules } = await input.index();

    // read the test module's declarations, its symbols and the edges from its test from the index
    const indexed = code.find((each) => each.path === "src/note.test.ts");
    const module = present(
        modules.find((each) => each.path === "src/note.test.ts"),
        "the test module's graph",
    );
    const registered = module.declarations.find((declaration) => declaration.kind === "test");
    expect({
        // declare each test and suite at the module, which defines no symbols
        symbols: indexed?.symbols,
        declarations: module.declarations.map(({ moniker, symbol, kind, name }) => ({
            moniker: local(moniker),
            symbol: local(symbol),
            kind,
            name,
        })),
        covers: (indexed?.edges ?? [])
            .filter((edge) => edge.from === registered?.moniker && edge.kind === "covers")
            .map((edge) => edge.to.slice(edge.to.indexOf("/") + 1)),
    }).toEqual({
        symbols: [],
        declarations: [
            {
                moniker: "src/note.test.ts:suite:note",
                symbol: "src/note.test.ts",
                kind: "suite",
                name: "note",
            },
            {
                moniker: "src/note.test.ts:test:note › starts incomplete",
                symbol: "src/note.test.ts",
                kind: "test",
                name: "note › starts incomplete",
            },
        ],
        covers: ["src/note.ts#createNote"],
    });
});

test.concurrent("declare each example showing its symbol with controls and the properties its module evaluates from imported constants, and each scenario with its interaction and steps covering what its examples show", async ({
    expect,
}) => {
    // add a card component, an interaction, an example of the card and a scenario given that example
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    await using input = await Fixture.open("library", environment);
    await writeCards(input.source, files);
    await using build = await input.build(library.request);
    const { modules } = await input.index();

    // read the example module's declarations and edges, with monikers within the package
    const module = present(
        modules.find((each) => each.path === "src/card.example.ts"),
        "the example module's graph",
    );
    const loud = module.declarations.find((declaration) => declaration.kind === "example");
    expect({
        declarations: module.declarations.map(({ moniker, kind, name, description }) => ({
            moniker: local(moniker),
            kind,
            name,
            description,
        })),
        edges: module.edges
            .filter((edge) => edge.kind === "shows" || edge.kind === "covers")
            .map((edge) => [local(edge.from), edge.kind, local(edge.to)]),
    }).toEqual({
        declarations: [
            {
                moniker: "src/card.example.ts#noteCardComplete:scenario:complete",
                kind: "scenario",
                name: "complete",
                description: {
                    description: "complete a card",
                    interaction: graph.Moniker.of({
                        packageId: PackageManifest.package(build.manifest).id,
                        module: "src/card.interaction.ts",
                        name: "cardInteraction",
                    }),
                    examples: [loud?.moniker],
                    globals: { width: 320 },
                    when: [{ action: "done" }, { action: "set", properties: { tone: "quiet" } }],
                    then: {
                        observe: { state: { kind: "state" } },
                        end: { state: "done" },
                    },
                },
            },
            {
                moniker: "src/card.example.ts#noteCardLoud:example:loud",
                kind: "example",
                name: "loud",
                description: {
                    description: "a loud card",
                    properties: { title: "Groceries", tone: "loud", lines: -2 },
                    objects: {},
                    controls: [
                        {
                            property: "title",
                            label: "The note's title.",
                            isOptional: false,
                            input: { kind: "text" },
                        },
                        {
                            property: "isComplete",
                            label: "Whether the note is done.",
                            isOptional: true,
                            input: { kind: "boolean" },
                        },
                        {
                            property: "tone",
                            label: "The card's look.",
                            isOptional: true,
                            input: { kind: "select", options: ["quiet", "loud"] },
                        },
                        {
                            property: "lines",
                            label: "The lines of text shown.",
                            isOptional: true,
                            input: { kind: "number" },
                        },
                    ],
                },
            },
        ],
        edges: [
            [
                "src/card.example.ts#noteCardComplete:scenario:complete",
                "covers",
                "src/card.ts#NoteCard",
            ],
            ["src/card.example.ts#noteCardLoud:example:loud", "shows", "src/card.ts#NoteCard"],
        ],
    });
});
