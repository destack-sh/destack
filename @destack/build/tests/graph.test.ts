import { test } from "@destack/test";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { graph } from "@destack/package";
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

/** A loud card. */
export const LoudCard = defineExample({
    of: NoteCard,
    name: "loud",
    properties: { title: "Groceries", tone: "loud", lines: -2 },
    render: (properties) => NoteCard({ title: "Groceries", ...properties }),
});

/** Complete a loud card. */
export const CompleteCard = defineScenario({
    interaction: cardInteraction,
    name: "complete a card",
    given: { examples: [LoudCard], environment: { width: 320 } },
    when: [{ action: "done" }, { action: "set", properties: { tone: "quiet" } }],
    then: {
        observe: { state: { kind: "state" } },
        end: { state: "done" },
    },
});
`;

/** Add a card component, an interaction of cards, an example of the card and a scenario taking steps on it. */
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

    // keep the untouched module's file byte for byte, and change the edited module's file and the root
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

test.concurrent("declare each test in its module's graph covering the package symbols it calls", async ({
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

    // read the test module's declarations and the edges from its test
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
        declarations: module.declarations.map(({ kind, name }) => ({ kind, name })),
        covers: module.edges
            .filter((edge) => edge.from === registered?.moniker && edge.kind === "covers")
            .map((edge) => edge.to.slice(edge.to.indexOf("/") + 1)),
    }).toEqual({
        declarations: [
            { kind: "test", name: "note › starts incomplete" },
            { kind: "suite", name: "note" },
        ],
        covers: ["src/note.ts#createNote"],
    });
});

test.concurrent("declare each example showing its symbol with controls, and each scenario with its interaction and steps covering what its examples show", async ({
    expect,
}) => {
    // add a card component, an interaction, an example of the card and a scenario given that example
    await using input = await Fixture.open("library");
    await writeCards(input.source);
    await using build = await input.build(library.request);

    // read the example module's declarations and edges, naming monikers within the package
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
                moniker: "src/card.example.ts#CompleteCard:scenario",
                kind: "scenario",
                name: "complete a card",
                description: {
                    interaction: graph.Moniker.of({
                        packageId: build.manifest.package.id,
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
                moniker: "src/card.example.ts#LoudCard:example",
                kind: "example",
                name: "loud",
                description: {
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
            ["src/card.example.ts#CompleteCard:scenario", "covers", "src/card.ts#NoteCard"],
            ["src/card.example.ts#LoudCard:example", "shows", "src/card.ts#NoteCard"],
        ],
    });
});
