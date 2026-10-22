import { test } from "@destack/test";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { graph } from "@destack/package";
import { PackageManifest } from "@destack/package/manifest";
import { schema } from "@destack/schema";
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
    environment: schema.object({ width: schema.number().exactOptional() }),
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
    given: { examples: [noteCardLoud], environment: { width: 320 } },
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
async function writeCards(source: string): Promise<void> {
    // depend on the declaring packages
    const manifest = join(source, "package.json");
    const declared = schema.looseObject({}).parse(JSON.parse(await readFile(manifest, "utf8")));
    await writeFile(
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
    await writeFile(join(source, "src/card.ts"), CARD);
    await writeFile(join(source, "src/card.interaction.ts"), CARD_INTERACTION);
    await writeFile(join(source, "src/card.look.ts"), CARD_LOOK);
    await writeFile(join(source, "src/card.example.ts"), CARD_EXAMPLES);
}

test.concurrent("reuse the graph file of each module an edit leaves unchanged", async ({
    expect,
}) => {
    // build the library, then reorder a function body in one module and build again
    await using input = await Fixture.open("library");
    await using before = await input.build(library.request);
    const note = join(input.source, "src/note.ts");
    const source = await readFile(note, "utf8");
    await writeFile(
        note,
        source.replace("return { title, complete: false };", "return { complete: false, title };"),
    );
    await using after = await input.build(library.request);

    // leave the untouched module's file byte for byte, and change the edited module's file and the root
    const [first, second] = await Promise.all([before.reader.graph(), after.reader.graph()]);
    const index = await readFile(
        join(after.directory, `graph/${second.modules["src/index.ts"]}.json`),
    );
    expect([
        second.modules["src/index.ts"] === first.modules["src/index.ts"],
        second.modules["src/note.ts"] === first.modules["src/note.ts"],
        after.manifest.lists.graph.digest === before.manifest.lists.graph.digest,
        index.equals(
            await readFile(join(before.directory, `graph/${first.modules["src/index.ts"]}.json`)),
        ),
    ]).toEqual([true, false, false, true]);
});

test.concurrent("declare each test in its module's graph, and index the package symbols it calls as covered", async ({
    expect,
}) => {
    // add a suite whose test calls the library's note constructor
    await using input = await Fixture.open("library");
    const manifest = join(input.source, "package.json");
    const declared = schema.looseObject({}).parse(JSON.parse(await readFile(manifest, "utf8")));
    await writeFile(
        manifest,
        JSON.stringify({ ...declared, devDependencies: { "@destack/test": "workspace:*" } }),
    );
    await writeFile(
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
    await using build = await input.build(library.request);
    const code = await input.index();

    // read the test module's declarations from the build, and its symbols and the edges from its test from the code index
    const indexed = code.find((each) => each.path === "src/note.test.ts");
    const root = await build.reader.graph();
    const module = graph.Module.parse(
        JSON.parse(
            await readFile(
                join(build.directory, `graph/${root.modules["src/note.test.ts"]}.json`),
                "utf8",
            ),
        ),
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
    await using input = await Fixture.open("library");
    await writeCards(input.source);
    await using build = await input.build(library.request);

    // read the example module's declarations and edges, with monikers within the package
    const root = await build.reader.graph();
    const module = graph.Module.parse(
        JSON.parse(
            await readFile(
                join(build.directory, `graph/${root.modules["src/card.example.ts"]}.json`),
                "utf8",
            ),
        ),
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
                    environment: { width: 320 },
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
