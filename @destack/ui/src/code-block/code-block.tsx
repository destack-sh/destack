import { type Grammar, type LineRange, Source } from "@destack/code";
import { t } from "@destack/locale";
import { state } from "@destack/theme/state";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { text } from "@destack/theme/text";
import { color, font, radius, size, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import {
    createMemo,
    createSignal,
    For,
    type JSX,
    omit,
    Show,
    untrack,
    useLocale,
} from "@destack/view";
import { CodeSyntax, CodeTokens, isRenderedAtOnce, onNear } from "../code/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The lines of a chunk, rendered together once they come near the screen, about two screens. */
const CHUNK_LINES = 80;

/** The room of a line number, as wide as the longest one. */
const NUMBER_WIDTH = "calc(var(--code-digits) * 1ch + 2.25em)";

/** The styles of a code block and its elements. */
const styles = style.create({
    block: {
        display: "flex",
        flexDirection: "column",
        minWidth: 0,
        maxWidth: "100%",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.background,
        color: color.foreground,
        overflow: "hidden",
    },
    header: {
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: space[2],
        minHeight: size[3],
        paddingInlineStart: space[3],
        paddingInlineEnd: space[1],
        borderBottomWidth: stroke.border,
        borderColor: color.border,
        color: color.mutedForeground,
    },
    title: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
        whiteSpace: "nowrap",
    },
    content: {
        overflowX: "auto",
        fontFamily: font.code,
        paddingBlock: space[3],
    },
    code: {
        display: "grid",
        minWidth: "max-content",
    },
    chunk: { display: "grid" },
    row: {
        display: "flex",
        minBlockSize: "1lh",
        whiteSpace: "pre",
    },
    wrapping: { minWidth: 0 },
    wrapped: { overflowWrap: "anywhere", whiteSpace: "pre-wrap" },
    text: { flexGrow: 1, minWidth: 0, paddingInline: space[3] },
    beside: { paddingInlineStart: 0 },
    number: {
        position: "sticky",
        insetInlineStart: 0,
        boxSizing: "border-box",
        flexShrink: 0,
        width: NUMBER_WIDTH,
        paddingInlineStart: space[3],
        paddingInlineEnd: space[3],
        backgroundColor: color.background,
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
        textAlign: "end",
        userSelect: "none",
    },
    pickable: { cursor: "pointer" },
    marked: {
        outlineStyle: { default: null, [media.forcedColors]: "solid" },
        outlineWidth: { default: null, [media.forcedColors]: "2px" },
        outlineOffset: { default: null, [media.forcedColors]: "-2px" },
        outlineColor: { default: null, [media.forcedColors]: "Highlight" },
        backgroundColor: `color-mix(in oklab, ${swatch.accentTint} 45%, ${color.background})`,
        boxShadow: `inset 3px 0 0 ${swatch.accentMark}`,
    },
    markedNumber: {
        backgroundColor: {
            default: `color-mix(in oklab, ${swatch.accentTint} 70%, ${color.background})`,
            [media.forcedColors]: "Highlight",
        },
        boxShadow: `inset 3px 0 0 ${swatch.accentMark}`,
        color: { default: swatch.accentTintText, [media.forcedColors]: "HighlightText" },
        forcedColorAdjust: { default: null, [media.forcedColors]: "none" },
    },
});

/** The properties of an element of a code block, the native element's attributes included. */
export type CodeBlockElementProperties<Attributes> = Omit<Attributes, "class"> &
    ElementPartProperties;

/** Render a block of code with an optional header, as a figure its title captions. */
export function CodeBlock(
    properties: CodeBlockElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    return renderPart("figure", "code-block", properties, styles.block);
}

/** Render the bar above the code with its title and actions, such as a copy button. */
export function CodeBlockHeader(
    properties: CodeBlockElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return renderPart("div", "code-block-header", properties, [text.xs, styles.header]);
}

/** Render the code's title, such as its file name, captioning the figure. */
export function CodeBlockTitle(
    properties: CodeBlockElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    return renderPart("figcaption", "code-block-title", properties, styles.title);
}

/** The properties of a code block's code, the native `pre` element's attributes included. */
export type CodeBlockContentProperties = CodeBlockElementProperties<
    Omit<JSX.HTMLAttributes<HTMLPreElement>, "onSelectionChange" | "children">
> & {
    /** Whether each line shows its number. */
    readonly isNumbered?: boolean;
    /** The accessible name of the scrolling region, its title when it has one. */
    readonly label?: string;
    /** The code. */
    readonly code: string;
    /** The grammar that colors the code, which shows plain without one. */
    readonly grammar?: Grammar | undefined;
    /** Whether long lines scroll or wrap, scrolling by default. */
    readonly overflow?: "scroll" | "wrap";
    /** The number of the first line, for code from within a file, one by default. */
    readonly start?: number;
    /** The lines that stand out, counted from the first as zero. */
    readonly marked?: readonly LineRange[];
    /** The selected lines, counted from the first as zero. */
    readonly selection?: LineRange | undefined;
    /** Handle the lines a person selects by their numbers. */
    readonly onSelectionChange?: (selection: LineRange) => void;
};

/** Render the lines of some code in its grammar's colors. */
export function CodeBlockContent(properties: CodeBlockContentProperties): JSX.Element {
    const locale = useLocale();
    const rest = omit(
        properties,
        "isNumbered",
        "label",
        "code",
        "grammar",
        "overflow",
        "start",
        "marked",
        "selection",
        "onSelectionChange",
    );

    return renderPart(
        "pre",
        "code-block-content",
        rest,
        [text.sm, state.focusInset, styles.content],
        {
            dir: "ltr",
            tabindex: 0,
            get "aria-label"() {
                return properties.label ?? locale.render(t.context("title")`Code`);
            },
            get children() {
                return <CodeLines {...properties} />;
            },
        },
    );
}

/** Render the lines of some code, chunk by chunk as they come near the screen. */
function CodeLines(properties: CodeBlockContentProperties): JSX.Element {
    // parse the code as far as its lines show
    const source = createMemo(() => Source.of(properties.code));
    const syntax = new CodeSyntax(source, () => properties.grammar);
    const chunks = (): number => Math.ceil(source().lines / CHUNK_LINES);
    const digits = (): number =>
        Math.max(2, String((properties.start ?? 1) + source().lines - 1).length);
    let anchor: number | undefined;
    const select = (line: number, isExtended: boolean): void => {
        // select a line, or those up to it from where the selection started
        const from = isExtended ? (anchor ?? properties.selection?.start ?? line) : line;
        anchor = from;
        properties.onSelectionChange?.({
            start: Math.min(from, line),
            end: Math.max(from, line) + 1,
        });
    };

    return (
        <code
            onPointerDown={(event) => {
                const line = lineOf(event.target);
                if (line !== undefined && event.button === 0 && properties.onSelectionChange) {
                    event.preventDefault();
                    select(line, event.shiftKey);
                }
            }}
            onPointerOver={(event) => {
                // take the lines a drag passes
                const line = lineOf(event.target);
                if (line !== undefined && event.buttons === 1 && anchor !== undefined) {
                    select(line, true);
                }
            }}
            {...style.attributes([styles.code, properties.overflow === "wrap" && styles.wrapping], {
                "--code-digits": String(digits()),
            })}
        >
            <For each={Array.from({ length: chunks() }, (_, chunk) => chunk)}>
                {(chunk) => (
                    <CodeChunk
                        chunk={chunk}
                        count={Math.min(CHUNK_LINES, source().lines - chunk * CHUNK_LINES)}
                        syntax={syntax}
                        block={properties}
                    />
                )}
            </For>
        </code>
    );
}

/** Read the line whose number an event is on, absent elsewhere. */
function lineOf(target: EventTarget | null): number | undefined {
    const number =
        target instanceof Element ? target.closest<HTMLElement>("[data-line-number]") : null;
    const line = number?.dataset["line"];

    return line === undefined ? undefined : Number(line);
}

/** Render a chunk of lines, or the room they take while they are far from the screen. */
function CodeChunk(properties: {
    /** Its place among the chunks. */
    readonly chunk: number;
    /** The lines it has. */
    readonly count: number;
    /** The code. */
    readonly syntax: CodeSyntax;
    /** What the block shows of each line. */
    readonly block: CodeBlockContentProperties;
}): JSX.Element {
    // render it once it comes near, and leave it there
    const chunk = untrack(() => properties.chunk);
    const [isNear, setNear] = createSignal(chunk === 0 || isRenderedAtOnce(), { ownedWrite: true });
    const lines = createMemo(() =>
        Array.from({ length: properties.count }, (_, index) => chunk * CHUNK_LINES + index),
    );
    let element: HTMLSpanElement | undefined;
    onNear(
        () => element,
        () => setNear(true),
    );

    return (
        <span
            ref={(created) => (element = created)}
            data-slot="code-block-chunk"
            {...style.attributes(
                [styles.chunk],
                isNear() ? undefined : { "min-height": `${String(properties.count)}lh` },
            )}
        >
            <Show when={isNear()}>
                <For each={lines()}>
                    {(line) => (
                        <CodeRow line={line} syntax={properties.syntax} block={properties.block} />
                    )}
                </For>
            </Show>
        </span>
    );
}

/** Render one line of code, with its number when its block numbers lines. */
function CodeRow(properties: {
    /** The line. */
    readonly line: number;
    /** The code. */
    readonly syntax: CodeSyntax;
    /** What the block shows of each line. */
    readonly block: CodeBlockContentProperties;
}): JSX.Element {
    // find whether it stands out
    const { line, block } = untrack(() => ({ line: properties.line, block: properties.block }));
    const isWithin = ({ start, end }: LineRange): boolean => line >= start && line < end;
    const isMarked = (): boolean =>
        (block.marked ?? []).some(isWithin) ||
        (block.selection !== undefined && isWithin(block.selection));
    const tokens = createMemo(() => [...properties.syntax.tokens(line)]);

    return (
        <span
            data-slot="code-block-line"
            data-marked={isMarked() ? "" : undefined}
            {...style.attrs(
                styles.row,
                block.overflow === "wrap" && styles.wrapped,
                isMarked() && styles.marked,
            )}
        >
            <Show when={block.isNumbered === true}>
                <span
                    aria-hidden="true"
                    data-line={String(line)}
                    data-line-number={String((block.start ?? 1) + line)}
                    {...style.attrs(
                        styles.number,
                        block.onSelectionChange !== undefined && styles.pickable,
                        isMarked() && styles.markedNumber,
                    )}
                >
                    {String((block.start ?? 1) + line)}
                </span>
            </Show>
            <span {...style.attrs(styles.text, block.isNumbered === true && styles.beside)}>
                <CodeTokens tokens={tokens()} />
            </span>
        </span>
    );
}
