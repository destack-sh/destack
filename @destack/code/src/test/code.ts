import type { Clock } from "@destack/environment";
import type { Simulation, SimulatedTelemetry } from "@destack/environment/simulation";
import type { SetStep } from "@destack/package/declare";
import { type JsonValue, present } from "@destack/schema";
import type { Driver } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { Difference } from "../difference/index.ts";
import type { Syntax } from "../grammar/index.ts";
import tsx from "../grammar/language/tsx.ts";
import { Merge } from "../merge/index.ts";
import { type CodeObservation, type CodeStep, codeInteraction } from "../scenario/code.ts";
import { Source } from "../source/index.ts";
import { Random } from "./random.ts";

/** A view, a line of which each line of a source repeats in turn. */
const VIEW = [
    'import { t } from "@destack/locale";',
    "/** Render a note with its title and how long it takes to read. */",
    "export function NoteView(properties: { readonly id: string }): JSX.Element {",
    "    // follow the note as anyone changes it",
    "    const [current, { update }] = useObject(note, () => properties.id);",
    "    const minutes = () => Math.ceil(current().words / 220);",
    "",
    "    return (",
    "        <article data-long={minutes() > 4} onClick={() => update({ isOpen: true })}>",
    "            <h1>{current().title ?? t`Untitled`}</h1>",
    '            <Badge variant="outline">{`${minutes()} min`}</Badge>',
    "        </article>",
    "    );",
    "}",
    "",
];

/** The lines a screen shows. */
const SCREEN_LINES = 60;

/** A driver that plays code scenarios on one source in memory. */
export class CodeDriver implements Driver<CodeStep, CodeObservation> {
    /** The interaction the driver speaks. */
    static readonly interaction = codeInteraction;
    /** The settings of the worlds: none change the numbers of code. */
    static readonly configuration = {};

    /** The world whose clock and telemetry the scenario reads. */
    readonly #simulation: Simulation;
    /** The source. */
    #source = Source.of("");
    /** The source as it was parsed last. */
    #syntax: Syntax | undefined;
    /** The changes found last. */
    #changes = 0;
    /** The random draws, the same every play. */
    readonly #random = new Random(1);

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

    /** Start a world with an empty source. */
    static start(): CodeDriver {
        return new CodeDriver(testSimulation());
    }

    /** Close the world. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#simulation[Symbol.asyncDispose]();
    }

    /** Take one step on the source. */
    act(step: SetStep | CodeStep): void {
        // refuse example properties
        if (step.action === "set") {
            throw new TypeError("a code world takes no example properties");
        }

        // run the operation
        if (step.action === "write") {
            this.#source = Source.of(
                Array.from(
                    { length: step.lines },
                    (_, line) => `${VIEW[line % VIEW.length] ?? ""}\n`,
                ).join(""),
            );
        } else if (step.action === "parse") {
            const end = step.extent === "screen" ? this.#source.start(SCREEN_LINES) : undefined;
            this.#syntax = tsx.parse(this.#source, end);
        } else if (step.action === "type") {
            this.#type();
        } else if (step.action === "tokens") {
            const line = this.#random.below(this.#source.lines);
            const from = this.#source.start(line);
            present(this.#syntax, "the parsed source").tokens({
                from,
                to: this.#source.start(line + SCREEN_LINES),
            });
        } else if (step.action === "compare") {
            const difference = Difference.of(this.#source, this.#changed(step.share));
            this.#changes = difference.changes.length;
        } else {
            const [ours, theirs] = [this.#changed(step.share), this.#changed(step.share)];
            this.#changes = Merge.of(this.#source, ours, theirs).chunks.length;
        }
    }

    /** Count the source's lines, or the changes found last. */
    observe(observation: CodeObservation): JsonValue {
        return observation.count === "lines" ? this.#source.lines : this.#changes;
    }

    /** Read the fingerprint of the world: its characters and its changes, equal across plays. */
    fingerprint(): number {
        return this.#source.text.length * 1000 + this.#changes;
    }

    /** Type a character within a random word. */
    #type(): void {
        // find the word, and type inside it
        const word = this.#source.text.indexOf(
            "current",
            this.#random.below(this.#source.text.length),
        );
        const at = Math.max(0, word) + 1;
        this.#syntax = present(this.#syntax, "the parsed source").edit({
            from: at,
            to: at,
            insert: "q",
        });
        this.#source = this.#syntax.source;
    }

    /** Change some lines of a hundred. */
    #changed(share: number): Source {
        const lines = this.#source.text
            .split("\n")
            .map((line) =>
                this.#random.happens(share)
                    ? `${line} // ${String(this.#random.below(1e6))}`
                    : line,
            );

        return Source.of(lines.join("\n"));
    }
}
