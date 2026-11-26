import { Difference, type FilePatch, type Grammar, Source } from "@destack/code";
import { type Accessor, createMemo, createSignal, type Setter, untrack } from "@destack/view";
import { CodeSyntax, isRenderedAtOnce } from "../code/index.ts";
import {
    type DiffFold,
    type DiffLine,
    DiffModel,
    type DiffPair,
    type DiffRow,
    type DiffSide,
    pairsOf,
    type Revealed,
} from "./model.ts";

/** The unchanged lines shown on either side of a change. */
const CONTEXT_LINES = 3;

/** The lines a press on a fold reveals. */
export const REVEALED_LINES = 20;

/** The rows of a block, about a screen. */
const BLOCK_ROWS = 40;

/** Lines of one side of a diff. */
export interface DiffSelection {
    /** Whose lines. */
    readonly side: DiffSide;
    /** The first line. */
    readonly start: number;
    /** The line after the last. */
    readonly end: number;
}

/** Whether a diff shows both texts in one column, or the old one beside the new one. */
export type DiffLayout = "unified" | "split";

/** A line of a diff on the page. */
export interface DiffPlace {
    /** Its row. */
    readonly row: HTMLElement;
    /** Whose line. */
    readonly side: DiffSide;
    /** The line, counted from zero. */
    readonly line: number;
}

/** What a diff compares and how. */
export interface DiffOptions {
    /** The old text, none for an added file. */
    readonly before?: string | Source;
    /** The new text, none for a removed file. */
    readonly after?: string | Source;
    /** The changes of a file, in place of its texts. */
    readonly patch?: FilePatch;
    /** The grammar that colors the texts, which show plain without one. */
    readonly grammar?: Grammar | undefined;
    /** Whether space counts, all of it by default. */
    readonly whitespace: "exact" | "trailing" | "ignored";
    /** How the texts are laid out, in one column by default. */
    readonly layout: DiffLayout;
    /** The unchanged lines shown on either side of a change, three by default. */
    readonly context?: number;
    /** The selected lines, for a diff whose owner controls them. */
    readonly selection?: DiffSelection | undefined;
    /** Handle selected lines, none once nothing is. */
    readonly onSelectionChange?: (selection: DiffSelection | undefined) => void;
}

/** What a diff shows, how far it is revealed, and which lines are selected. */
export class DiffControl {
    /** The options, read as they change. */
    readonly options: DiffOptions;
    /** The texts and what changed. */
    readonly model: Accessor<DiffModel>;
    /** The rows, one column's or two's, in blocks rendered together. */
    readonly blocks: Accessor<readonly (readonly (DiffRow | DiffPair)[])[]>;
    /** Whether every block is rendered at once, for markup a server renders. */
    readonly #isAtOnce: boolean;
    /** The blocks that came near the screen. */
    readonly near: Accessor<ReadonlySet<number>>;
    /** Replace the blocks that came near the screen. */
    readonly #setNear: Setter<ReadonlySet<number>>;
    /** Whether the lines are hidden under the header. */
    readonly isCollapsed: Accessor<boolean>;
    /** Replace whether the lines are hidden. */
    readonly setCollapsed: Setter<boolean>;
    /** Whether the texts stand beside each other. */
    readonly isSplit: Accessor<boolean>;
    /** The line the person is at, absent for none. */
    readonly active: Accessor<DiffPlace | undefined>;
    /** Replace the line the person is at. */
    readonly setActive: Setter<DiffPlace | undefined>;
    /** The lines selected here while the owner does not control them. */
    readonly #selection: Accessor<DiffSelection | undefined>;
    /** Replace the lines selected here. */
    readonly #setSelection: Setter<DiffSelection | undefined>;
    /** The line the selection extends from, absent before a person starts one. */
    #anchor: { readonly side: DiffSide; readonly line: number } | undefined;
    /** What is revealed of each fold. */
    readonly #setRevealed: Setter<Revealed>;
    /** The text of each side, parsed as far as its lines show. */
    readonly syntax: Readonly<Record<DiffSide, CodeSyntax>>;

    /** Follow a diff's options, in one column where there is no room for two. */
    constructor(options: DiffOptions, isNarrow: Accessor<boolean>) {
        // compare the texts, or read the patch
        const [revealed, setRevealed] = createSignal<Revealed>(new Map(), { ownedWrite: true });
        const [selection, setSelection] = createSignal<DiffSelection | undefined>(undefined, {
            ownedWrite: true,
        });
        const [isCollapsed, setCollapsed] = createSignal(false, { ownedWrite: true });
        const [near, setNear] = createSignal<ReadonlySet<number>>(new Set([0]), {
            ownedWrite: true,
        });
        const [active, setActive] = createSignal<DiffPlace | undefined>(undefined, {
            ownedWrite: true,
        });
        this.options = options;
        this.isSplit = () => options.layout === "split" && !isNarrow();
        this.active = active;
        this.setActive = setActive;
        this.model = createMemo(() =>
            options.patch === undefined
                ? new DiffModel(
                      Difference.of(sourceOf(options.before), sourceOf(options.after), {
                          whitespace: options.whitespace,
                      }),
                  )
                : DiffModel.ofPatch(options.patch),
        );
        this.blocks = createMemo(() => {
            const lines = this.model().rows(options.context ?? CONTEXT_LINES, revealed());
            const rows = this.isSplit() ? pairsOf(lines) : lines;

            return Array.from({ length: Math.ceil(rows.length / BLOCK_ROWS) }, (_, block) =>
                rows.slice(block * BLOCK_ROWS, (block + 1) * BLOCK_ROWS),
            );
        });
        this.#isAtOnce = isRenderedAtOnce();
        this.near = near;
        this.#setNear = setNear;
        this.isCollapsed = isCollapsed;
        this.setCollapsed = setCollapsed;
        this.#selection = selection;
        this.#setSelection = setSelection;
        this.#setRevealed = setRevealed;

        // parse each side as far as its lines show
        const syntaxOf = (side: DiffSide): CodeSyntax =>
            new CodeSyntax(
                () => this.model()[side],
                () => options.grammar,
            );
        this.syntax = { before: syntaxOf("before"), after: syntaxOf("after") };
    }

    /** The selected lines, absent for none. */
    selection(): DiffSelection | undefined {
        return "selection" in this.options ? this.options.selection : this.#selection();
    }

    /** Report whether a line is selected. */
    isSelected(line: DiffLine, side: DiffSide): boolean {
        const selection = this.selection();
        const place = line[side];

        return (
            selection?.side === side &&
            place !== undefined &&
            place >= selection.start &&
            place < selection.end
        );
    }

    /** Select lines, or none. */
    select(selection: DiffSelection | undefined): void {
        // forget where it started once nothing is
        if (selection === undefined) {
            this.#anchor = undefined;
        }
        this.#setSelection(() => selection);
        this.options.onSelectionChange?.(selection);
    }

    /** Select a line, from which the selection extends. */
    start(side: DiffSide, line: number): void {
        this.#anchor = { side, line };
        this.select({ side, start: line, end: line + 1 });
    }

    /** Select from the line the selection started at to another of its side. */
    extend(side: DiffSide, line: number): void {
        // start anew on the other side
        const selection = untrack(() => this.selection());
        const started =
            this.#anchor ?? (selection && { side: selection.side, line: selection.start });
        if (started?.side === side) {
            const { line: anchor } = started;
            this.select({ side, start: Math.min(anchor, line), end: Math.max(anchor, line) + 1 });
        } else {
            this.start(side, line);
        }
    }

    /** Report whether a block is rendered. */
    isShown(block: number): boolean {
        return this.#isAtOnce || this.near().has(block);
    }

    /** Render a block from now on. */
    show(block: number): void {
        this.#setNear((current) => (current.has(block) ? current : new Set(current).add(block)));
    }

    /** Render the block a change starts in. */
    showChange(place: number): void {
        // find the block
        const block = this.blocks().findIndex((rows) =>
            rows.some((row) => {
                // read the line of either text
                const pair = row.kind === "pair" ? (row.old ?? row.next) : undefined;
                const line = row.kind === "line" ? row.line : pair;

                return line?.place === place;
            }),
        );
        if (block !== -1) {
            this.show(block);
        }
    }

    /** Reveal some lines of a fold, or all of them. */
    reveal(fold: DiffFold, direction: "above" | "below" | "all"): void {
        // add to what was revealed of it, which moves the lines
        this.setActive(undefined);
        this.#setRevealed((current) => {
            const shown = current.get(fold.key) ?? { above: 0, below: 0 };
            const all = fold.before.end - fold.before.start;

            return new Map(current).set(fold.key, {
                above:
                    shown.above +
                    (direction === "below" ? 0 : direction === "all" ? all : REVEALED_LINES),
                below: shown.below + (direction === "below" ? REVEALED_LINES : 0),
            });
        });
    }
}

/** Read a text as a source, an empty one for none. */
function sourceOf(text: string | Source | undefined): Source {
    return text instanceof Source ? text : Source.of(text ?? "");
}
