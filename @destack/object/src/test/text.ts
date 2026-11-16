import type { Clock } from "@destack/environment";
import type { Simulation, SimulatedTelemetry } from "@destack/environment/simulation";
import type { SetStep } from "@destack/package/declare";
import { type JsonValue, nth } from "@destack/schema";
import type { Driver } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { type TextObservation, type TextStep, textInteraction } from "../scenario/text.ts";
import {
    RICH_TEXT,
    TextRope,
    TextSchema,
    TextSequence,
    TextTypes,
    type TextEdit,
    type TextMarkType,
} from "../sequence/index.ts";

/** The words a text world types, cycled by its random draws. */
const WORDS = ["plan", "the", "release", "with", "notes", "and", "a", "review", "of", "tasks"];

/** The run of the world's editor. */
const SITE = "editor";

/** The multiplier of the world's random draws. */
const RANDOM_MULTIPLIER = 1_103_515_245;

/** The increment of the world's random draws. */
const RANDOM_INCREMENT = 12_345;

/** The modulus of the world's linear congruential random draws, two to the 31st. */
const RANDOM_MODULUS = 2_147_483_648;

/** The schema a text world's marks and nodes follow: rich text and overlapping highlights. */
const SCHEMA = TextSchema.of(
    ...RICH_TEXT,
    TextSchema.mark("highlight", { expand: "none", isMultiple: true }),
);

/** A driver that plays text scenarios on one rope in memory. */
export class TextDriver implements Driver<TextStep, TextObservation> {
    /** The interaction the driver speaks. */
    static readonly interaction = textInteraction;
    /** The settings of the worlds: none change a text's numbers. */
    static readonly configuration = {};

    /** The world whose clock and telemetry the scenario reads. */
    readonly #simulation: Simulation;
    /** The text as the store ordered every edit. */
    #rope = new TextRope();
    /** The marks and nodes created so far, naming each new one. */
    #counter = 0;
    /** The offset of the next element the editor types. */
    #clock = 0;
    /** The marks the store ordered so far, ordering the next. */
    #order = 0;
    /** The state of the world's random draws, the same every play. */
    #seed = 1;

    /** Take a started world. */
    constructor(simulation: Simulation) {
        this.#simulation = simulation;
    }

    /** The world's clock. */
    get clock(): Clock {
        return this.#simulation.clock;
    }

    /** The telemetry the world records. */
    get telemetry(): SimulatedTelemetry {
        return this.#simulation.telemetry;
    }

    /** Start a world with an empty text. */
    static start(): TextDriver {
        return new TextDriver(testSimulation());
    }

    /** Close the world. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#simulation[Symbol.asyncDispose]();
    }

    /** Take one step on the text. */
    act(step: SetStep | TextStep): void {
        // refuse example properties
        if (step.action === "set") {
            throw new TypeError("a text world takes no example properties");
        }

        // edit through the operation's family, or read the spans
        const { length } = this.#rope;
        if (step.action === "type") {
            const words = Array.from({ length: step.words }, () => this.#word()).join(" ");
            this.#change(this.#below(length + 1), 0, `${words} `);
        } else if (step.action === "append") {
            this.#change(length, 0, this.#characters(step.characters));
        } else if (step.action === "prepend") {
            this.#change(0, 0, this.#characters(step.characters));
        } else if (step.action === "delete") {
            const from = this.#below(Math.max(1, length - step.characters));
            this.#change(from, Math.min(step.characters, length - from), "");
        } else if (step.action === "mark") {
            this.#mark(step.type, length);
        } else if (step.action === "unmark") {
            this.#unmark();
        } else if (step.action === "node") {
            this.#node(length);
        } else if (step.action === "convert") {
            this.#convert();
        } else if (step.action === "spans") {
            this.#rope.read().spans(SCHEMA);
        } else {
            this.#merge(step.parties, step.changes);
        }
    }

    /** Count the text's visible characters, its pieces, its marks or its nodes. */
    observe(observation: TextObservation): JsonValue {
        // count over the rope's chunks
        const { chunks, length } = this.#rope;
        const runs = chunks.flatMap((chunk) => chunk.runs);
        const counts = {
            characters: length,
            pieces: runs.length,
            marks: chunks.reduce((sum, chunk) => sum + chunk.marks.length, 0),
            nodes: runs.filter((piece) => "node" in piece && TextSequence.isVisible(piece)).length,
        };

        return counts[observation.count];
    }

    /** Read the fingerprint of the text: its characters, pieces and marks, equal across plays. */
    fingerprint(): number {
        // count the pieces and marks of every chunk
        const { chunks, length } = this.#rope;
        const pieces = chunks.reduce((sum, chunk) => sum + chunk.runs.length, 0);
        const marks = chunks.reduce((sum, chunk) => sum + chunk.marks.length, 0);

        return this.#simulation.fingerprint + length + 7 * pieces + 13 * marks;
    }

    /** Replace visible characters as the editor's site, continuing its runs. */
    #change(from: number, length: number, insert: string): void {
        const next = { run: SITE, offset: this.#clock };
        this.#clock += insert.length;
        for (const edit of this.#rope.change({ from, to: from + length, insert }, next)) {
            this.#rope = this.#rope.apply(edit);
        }
    }

    /** Mark a range of a few words with a type, the store ordering it. */
    #mark(name: "bold" | "link" | "highlight", length: number): void {
        // mark nothing in a text too short for a range
        if (length < 2) {
            return;
        }

        // choose a range and order the mark
        const from = this.#below(length - 1);
        const to = Math.min(length, from + 1 + this.#below(15));
        const type: TextMarkType = name === "highlight" ? SCHEMA.markType(name) : TextTypes[name];
        const values = { bold: true, link: { href: "https://destack.sh" }, highlight: "ada" };
        this.#counter += 1;
        this.#order += 1;
        this.#rope = this.#rope.apply({
            mark: {
                id: `mark.${this.#counter}`,
                type: name,
                value: values[name],
                ...this.#rope.range(from, to, type),
                order: this.#order,
            },
        });
    }

    /** Remove a random mark, if any. */
    #unmark(): void {
        const marks = this.#rope.chunks.flatMap((chunk) => chunk.marks);
        if (marks.length > 0) {
            this.#rope = this.#rope.apply({ unmark: [nth(marks, this.#below(marks.length))] });
        }
    }

    /** Insert a paragraph marker at a random place. */
    #node(length: number): void {
        // insert after a random visible element, or at the start
        const offset = this.#below(length + 1);
        const after = offset === 0 ? undefined : this.#rope.element(offset - 1);
        this.#counter += 1;
        this.#rope = this.#rope.apply({
            insert: { type: "paragraph", value: null },
            run: `node.${this.#counter}`,
            start: 0,
            ...(after === undefined ? {} : { after }),
        });
    }

    /** Convert a random visible node into a heading, if any. */
    #convert(): void {
        const nodes = this.#rope.chunks.flatMap((chunk) =>
            chunk.runs.filter((piece) => "node" in piece),
        );
        if (nodes.length > 0) {
            const node = nth(nodes, this.#below(nodes.length));
            const level = 1 + this.#below(6);
            this.#rope = this.#rope.apply({
                set: { run: node.run, offset: node.start },
                node: { type: "heading", value: { level } },
            });
        }
    }

    /** Let parties edit the same text concurrently, then apply their edits in turn. */
    #merge(parties: number, changes: number): void {
        // edit a replica per party, each with its own run
        const base = this.#rope;
        this.#counter += 1;
        const pending = Array.from({ length: parties }, (_, party) => {
            // type words at random places
            let replica = base;
            const edits: TextEdit[] = [];
            const run = `party.${String(this.#counter)}.${String(party)}`;
            let clock = 0;
            for (let change = 0; change < changes; change += 1) {
                const at = this.#below(replica.length + 1);
                const insert = `${this.#word()} `;
                const made = replica.change({ from: at, to: at, insert }, { run, offset: clock });
                clock += insert.length;
                for (const edit of made) {
                    replica = replica.apply(edit);
                    edits.push(edit);
                }
            }

            return edits;
        });

        // apply every party's edits in the store's order
        for (const edits of pending) {
            for (const edit of edits) {
                this.#rope = this.#rope.apply(edit);
            }
        }
    }

    /** Draw the next word. */
    #word(): string {
        return nth(WORDS, this.#below(WORDS.length));
    }

    /** Draw characters of words up to a length. */
    #characters(count: number): string {
        let characters = "";
        while (characters.length < count) {
            characters += `${this.#word()} `;
        }

        return characters.slice(0, count);
    }

    /** Draw a whole number below a bound, the same draws every play. */
    #below(bound: number): number {
        this.#seed = (this.#seed * RANDOM_MULTIPLIER + RANDOM_INCREMENT) % RANDOM_MODULUS;

        return Math.floor((this.#seed / RANDOM_MODULUS) * bound);
    }
}
