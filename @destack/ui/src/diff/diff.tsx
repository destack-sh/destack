import arrowDown from "@destack/icon/phosphor/arrow-line-down";
import arrowUp from "@destack/icon/phosphor/arrow-line-up";
import arrowsOut from "@destack/icon/phosphor/arrows-out-line-vertical";
import caretDown from "@destack/icon/phosphor/caret-down";
import caretRight from "@destack/icon/phosphor/caret-right";
import check from "@destack/icon/phosphor/check";
import plus from "@destack/icon/phosphor/plus";
import xIcon from "@destack/icon/phosphor/x";
import type { Change } from "@destack/code";
import { argument, plural, t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { state } from "@destack/theme/state";
import { text } from "@destack/theme/text";
import {
    color,
    font,
    radius,
    shadow,
    size,
    space,
    stroke,
    swatch,
    weight,
} from "@destack/theme/tokens.stylex";
import { Icon } from "@destack/ui/icon";
import {
    createContext,
    createMemo,
    flush,
    For,
    handle,
    type JSX,
    omit,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { createMediaQuery } from "@destack/view/primitives/media";
import { ActionButton } from "../action/index.ts";
import { Badge } from "../badge/index.ts";
import { Button } from "../button/index.ts";
import { onNear } from "../code/index.ts";
import { CopyButton } from "../copy-button/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import { visuallyHiddenStyle } from "../visually-hidden/index.ts";
import {
    acceptChange,
    commentChange,
    nextChange,
    previousChange,
    rejectChange,
} from "./actions.ts";
import { DiffControl, type DiffOptions, type DiffPlace, REVEALED_LINES } from "./control.ts";
import { DiffCode } from "./lines.tsx";
import type { DiffFold, DiffLine, DiffPair, DiffRow, DiffSide } from "./model.ts";

/** A screen too narrow for two columns. */
const NARROW = "(width < 48rem)";

/** The height of a line of code in its font's size. */
const LINE_HEIGHT = 1.65;

/** The room of a line number, as wide as the longest one. */
const NUMBER_WIDTH = "calc(var(--diff-digits) * 1ch + 1.5em)";

/** The squares a file's counts fill. */
const SQUARES = 5;

/** The lines a removed change is hatched with, so more than its color tells it apart. */
const HATCH = `repeating-linear-gradient(-45deg, ${swatch.destructiveMark} 0 2px, color-mix(in oklab, ${swatch.destructiveMark} 35%, transparent) 2px 4px)`;

/** The styles of a diff, its header, its lines and its folds. */
const styles = style.create({
    diff: {
        minWidth: 0,
        maxWidth: "100%",
        overflow: "clip",
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.background,
        outlineStyle: "none",
    },
    header: {
        position: "sticky",
        insetBlockStart: 0,
        zIndex: 1,
        display: "flex",
        alignItems: "center",
        gap: space[2],
        minHeight: size[3],
        paddingBlock: space[1],
        paddingInlineStart: space[2],
        paddingInlineEnd: space[2],
        backgroundColor: color.background,
    },
    divided: {
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    path: {
        flexShrink: 1,
        minWidth: 0,
        overflow: "hidden",
        direction: "rtl",
        fontFamily: font.code,
        textAlign: "start",
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
        whiteSpace: "nowrap",
    },
    directory: { color: color.mutedForeground },
    file: { fontWeight: weight.medium },
    grow: { flexGrow: 1 },
    counts: {
        display: "inline-flex",
        flexShrink: 0,
        alignItems: "center",
        gap: space[2],
        fontFamily: font.code,
        fontVariantNumeric: "tabular-nums",
        fontWeight: weight.medium,
    },
    addedCount: { color: swatch.successText },
    removedCount: { color: swatch.destructiveText },
    squares: { display: "inline-flex", gap: `calc(${space[1]} / 2)` },
    square: {
        width: space[2],
        height: space[2],
        borderRadius: radius[1],
        backgroundColor: color.border,
    },
    addedSquare: { backgroundColor: swatch.successMark },
    removedSquare: { backgroundColor: swatch.destructiveMark },
    body: {
        containerType: "inline-size",
        overflowX: "auto",
        outlineStyle: "none",
        scrollbarWidth: "thin",
    },
    table: {
        width: "100%",
        borderCollapse: "collapse",
        borderSpacing: 0,
        fontFamily: font.code,
        fontSize: "0.93em",
        lineHeight: LINE_HEIGHT,
        tabSize: 4,
    },
    scrolling: { minWidth: "max-content" },
    split: { tableLayout: "fixed" },
    numberColumn: { width: NUMBER_WIDTH },
    number: {
        position: "sticky",
        zIndex: 1,
        boxSizing: "border-box",
        width: NUMBER_WIDTH,
        minWidth: NUMBER_WIDTH,
        paddingBlock: 0,
        paddingInlineStart: "0.9em",
        paddingInlineEnd: "0.6em",
        backgroundColor: color.background,
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
        textAlign: "end",
        userSelect: "none",
        verticalAlign: "top",
        cursor: "default",
    },
    firstNumber: { insetInlineStart: 0 },
    secondNumber: { insetInlineStart: NUMBER_WIDTH },
    pickable: {
        cursor: "pointer",
        color: {
            default: color.mutedForeground,
            ":hover": { default: null, [media.hover]: color.foreground },
        },
    },
    code: {
        width: "auto",
        paddingBlock: 0,
        paddingInlineStart: space[2],
        paddingInlineEnd: space[4],
        textAlign: "start",
        verticalAlign: "top",
        whiteSpace: "pre",
    },
    wrapped: {
        overflowWrap: "anywhere",
        whiteSpace: "pre-wrap",
    },
    half: {
        borderInlineStartWidth: stroke.border,
        borderInlineStartStyle: "solid",
        borderInlineStartColor: color.border,
    },
    bar: {
        position: "absolute",
        insetBlock: 0,
        insetInlineStart: 0,
        width: space[1],
        forcedColorAdjust: "none",
    },
    addedBar: { backgroundColor: swatch.successMark },
    removedBar: { backgroundImage: HATCH },
    added: {
        backgroundColor: `light-dark(color-mix(in oklab, ${swatch.successTint} 45%, ${color.background}), color-mix(in oklab, ${swatch.successTint} 30%, ${color.background}))`,
    },
    removed: {
        backgroundColor: `light-dark(color-mix(in oklab, ${swatch.destructiveTint} 45%, ${color.background}), color-mix(in oklab, ${swatch.destructiveTint} 30%, ${color.background}))`,
    },
    addedNumber: {
        backgroundColor: `light-dark(color-mix(in oklab, ${swatch.successTint} 70%, ${color.background}), color-mix(in oklab, ${swatch.successTint} 50%, ${color.background}))`,
        color: swatch.successTintText,
    },
    removedNumber: {
        backgroundColor: `light-dark(color-mix(in oklab, ${swatch.destructiveTint} 70%, ${color.background}), color-mix(in oklab, ${swatch.destructiveTint} 50%, ${color.background}))`,
        color: swatch.destructiveTintText,
    },
    selected: {
        backgroundImage: `linear-gradient(${swatch.accentTint}, ${swatch.accentTint})`,
        backgroundBlendMode: "normal",
    },
    missing: {
        backgroundColor: `color-mix(in oklab, ${color.muted} 55%, transparent)`,
    },
    fold: {
        paddingBlock: space[1],
        paddingInline: 0,
        backgroundColor: color.muted,
        color: color.mutedForeground,
    },
    foldLine: {
        position: "sticky",
        insetInlineStart: 0,
        display: "flex",
        alignItems: "center",
        gap: space[2],
        width: "fit-content",
        maxWidth: "100cqi",
        boxSizing: "border-box",
        paddingInline: space[2],
        fontFamily: font.text,
    },
    count: { flexShrink: 0, whiteSpace: "nowrap" },
    heading: {
        overflow: "hidden",
        minWidth: 0,
        fontFamily: font.code,
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
        whiteSpace: "nowrap",
    },
    comment: {
        position: "absolute",
        zIndex: 2,
        display: "grid",
        placeItems: "center",
        width: "1.3em",
        height: "1.3em",
        marginBlockStart: "0.1em",
        marginInlineStart: "-0.65em",
        borderRadius: radius[1],
        backgroundColor: swatch.accentSolid,
        color: swatch.accentLabel,
        cursor: "pointer",
    },
    frame: { position: "relative" },
    changeActions: {
        position: "absolute",
        insetInlineEnd: space[2],
        zIndex: 2,
        display: "flex",
        gap: `calc(${space[1]} / 2)`,
        padding: `calc(${space[1]} / 2)`,
        marginBlockStart: "-0.3rem",
        borderRadius: radius[2],
        backgroundColor: color.background,
        boxShadow: shadow.raised,
        fontFamily: font.text,
    },
    annotation: {
        padding: 0,
        borderBlockWidth: stroke.border,
        borderBlockStyle: "solid",
        borderBlockColor: color.border,
        backgroundColor: color.background,
        fontFamily: font.text,
        whiteSpace: "normal",
    },
    annotationBody: {
        position: "sticky",
        insetInlineStart: 0,
        width: "min(48rem, 100cqi)",
        boxSizing: "border-box",
        padding: space[3],
    },
    empty: {
        padding: space[4],
        color: color.mutedForeground,
        textAlign: "center",
    },
});

/** What a diff shows under a line, such as a comment. */
export interface DiffAnnotation {
    /** Whose line. */
    readonly side: DiffSide;
    /** The line, counted from zero. */
    readonly line: number;
    /** What it shows. */
    readonly content: () => JSX.Element;
}

/** The properties of a diff, with its own attributes. */
export interface DiffProperties
    extends
        DiffOptions,
        Omit<JSX.HTMLAttributes<HTMLElement>, "class" | "children" | "onSelectionChange">,
        Pick<ElementPartProperties, "xstyle"> {
    /** Whether long lines scroll or wrap, scrolling by default, wrapping beside each other. */
    readonly overflow?: "scroll" | "wrap";
    /** The path of the file, whose header shows it. */
    readonly path?: string;
    /** The path the file was moved from. */
    readonly previousPath?: string;
    /** What stands under lines, such as comments. */
    readonly annotations?: readonly DiffAnnotation[];
    /** Start a comment on a line. */
    readonly onComment?: (side: DiffSide, line: number) => void;
    /** Accept a change. */
    readonly onAccept?: (change: Change) => void | Promise<void>;
    /** Reject a change. */
    readonly onReject?: (change: Change) => void | Promise<void>;
    /** What labels it, its path by default. */
    readonly "aria-label"?: string;
    /** What stands at the end of its header, such as a menu. */
    readonly children?: JSX.Element;
}

/** A diff as its parts read it. */
interface DiffValue {
    /** What it shows. */
    readonly control: DiffControl;
    /** Its properties. */
    readonly properties: DiffProperties;
    /** How its texts are laid out. */
    readonly isSplit: () => boolean;
    /** Whether long lines wrap. */
    readonly isWrapped: () => boolean;
    /** Report whether a line's number takes a press. */
    readonly canSelect: () => boolean;
    /** What stands under lines, by side and line. */
    readonly annotations: () => ReadonlyMap<string, readonly DiffAnnotation[]>;
}

/** The diff around, null outside one. */
const DiffContext = createContext<DiffValue | null>(null);

/** Read the diff around, refusing a part outside one. */
function useDiff(): DiffValue {
    const diff = useContext(DiffContext);
    if (diff === null) {
        throw new TypeError("a diff part needs a diff around it");
    }

    return diff;
}

/** Render the changes of a file: its header, its changed lines and folds of the rest. */
export function Diff(properties: DiffProperties): JSX.Element {
    // follow what it shows, and go between its changes
    const isNarrow = createMediaQuery(NARROW);
    const control = new DiffControl(properties, isNarrow);
    const locale = useLocale();
    let root: HTMLElement | undefined;
    const rest = omit(
        properties,
        "before",
        "after",
        "patch",
        "grammar",
        "whitespace",
        "context",
        "selection",
        "onSelectionChange",
        "layout",
        "overflow",
        "path",
        "previousPath",
        "annotations",
        "onComment",
        "onAccept",
        "onReject",
        "aria-label",
        "children",
    );
    const annotations = createMemo(() =>
        Map.groupBy(properties.annotations ?? [], ({ side, line }) => `${side} ${String(line)}`),
    );
    const value: DiffValue = {
        annotations,
        control,
        properties,
        isSplit: () => control.isSplit(),
        isWrapped: () => control.isSplit() || properties.overflow === "wrap",
        canSelect: () =>
            properties.onSelectionChange !== undefined || properties.onComment !== undefined,
    };
    const path = (): string | undefined => properties.path ?? properties.patch?.path;
    const changeOf = (element: Element | null): Change | undefined => {
        const place = element?.closest("[data-change]")?.getAttribute("data-change");

        return place === null || place === undefined
            ? undefined
            : control.model().difference.changes[Number(place)];
    };
    const go = (place: number): void => {
        // render the change's lines, and focus the first
        control.setCollapsed(false);
        control.showChange(place);
        flush();
        const stop = root?.querySelector<HTMLElement>(
            `[data-change-start][data-change="${String(place)}"]`,
        );
        stop?.focus({ preventScroll: true });
        stop?.scrollIntoView({ block: "center" });
    };
    const step = (element: Element | null, by: 1 | -1): void => {
        // take the change after or before the one the person is at
        const { changes } = control.model().difference;
        const current = changeOf(element);
        const at =
            current === undefined ? (by === 1 ? -1 : changes.length) : changes.indexOf(current);
        if (at + by >= 0 && at + by < changes.length) {
            go(at + by);
        }
    };
    const decide = async (
        element: Element | null,
        settle: ((change: Change) => void | Promise<void>) | undefined,
    ): Promise<void> => {
        // settle the change, leaving the focus to the change in its place or the one after
        const change = changeOf(element);
        const { changes } = control.model().difference;
        if (change === undefined) {
            return;
        }
        const place = changes.indexOf(change);
        control.setActive(undefined);
        await settle?.(change);
        flush();
        const left = control.model().difference.changes.length;
        const next = left < changes.length ? place : place + 1;
        if (next < left) {
            go(next);
        } else {
            root?.querySelector<HTMLElement>("[data-slot=diff-content]")?.focus();
        }
    };
    const commented = (element: Element | null): { side: DiffSide; line: number } | undefined => {
        // take the last selected line, or the last line of the change
        const selection = control.selection();
        if (selection !== undefined) {
            return { side: selection.side, line: selection.end - 1 };
        }
        const change = changeOf(element);
        if (change === undefined) {
            return undefined;
        }
        const side: DiffSide = change.after.end > change.after.start ? "after" : "before";

        return { side, line: change[side].end - 1 };
    };
    const handlers = [
        handle(nextChange, { run: ({ element }) => step(element, 1) }),
        handle(previousChange, { run: ({ element }) => step(element, -1) }),
        handle(commentChange, {
            isEnabled: (element) =>
                properties.onComment !== undefined && commented(element) !== undefined,
            run: ({ element }) => {
                const target = commented(element);
                if (target !== undefined) {
                    properties.onComment?.(target.side, target.line);
                }
            },
        }),
        handle(acceptChange, {
            isEnabled: (element) =>
                properties.onAccept !== undefined && changeOf(element) !== undefined,
            run: ({ element }) => decide(element, properties.onAccept),
        }),
        handle(rejectChange, {
            isEnabled: (element) =>
                properties.onReject !== undefined && changeOf(element) !== undefined,
            run: ({ element }) => decide(element, properties.onReject),
        }),
    ];

    return (
        <DiffContext value={value}>
            {renderPart("section", "diff", rest, [text.sm, styles.diff], {
                ref: [(element: HTMLElement) => (root = element), ...handlers],
                get "aria-label"() {
                    return (
                        properties["aria-label"] ??
                        path() ??
                        locale.render(t.context("title")`Changes`)
                    );
                },
                get "data-layout"() {
                    return control.isSplit() ? "split" : "unified";
                },
                get children() {
                    return (
                        <>
                            <Show when={path()}>{(shown) => <DiffHeader path={shown()} />}</Show>
                            <Show when={!control.isCollapsed()}>
                                <DiffContent />
                            </Show>
                        </>
                    );
                },
            })}
        </DiffContext>
    );
}

/** Render a file's path, what happened to it and how many lines changed. */
function DiffHeader(properties: {
    /** The file's path. */
    readonly path: string;
}): JSX.Element {
    // split the path at its file, and count the lines
    const { control, properties: diff } = useDiff();
    const locale = useLocale();
    const cut = (): number => properties.path.lastIndexOf("/") + 1;
    const added = (): number => control.model().difference.added;
    const removed = (): number => control.model().difference.removed;
    const previous = (): string | undefined =>
        diff.previousPath ?? (diff.patch?.status === "renamed" ? diff.patch.before : undefined);
    const status = () => {
        // say what happened to the whole file
        const isAdded =
            diff.patch?.status === "added" ||
            (diff.patch === undefined && diff.before === undefined);
        const isRemoved =
            diff.patch?.status === "removed" ||
            (diff.patch === undefined && diff.after === undefined);

        return isAdded
            ? t.context("option")`Added`
            : isRemoved
              ? t.context("option")`Removed`
              : previous() === undefined
                ? undefined
                : t.context("option")`Moved`;
    };
    const squares = (): readonly ("added" | "removed" | "none")[] => {
        // fill the squares by the share of each, at least one for either there is
        const total = added() + removed();
        const green =
            total === 0
                ? 0
                : Math.min(
                      SQUARES - (removed() > 0 ? 1 : 0),
                      Math.max(added() > 0 ? 1 : 0, Math.round((added() / total) * SQUARES)),
                  );
        const filled = Math.min(SQUARES, Math.max(total === 0 ? 0 : 1, total));

        return Array.from({ length: SQUARES }, (_, place) =>
            place >= filled ? "none" : place < green ? "added" : "removed",
        );
    };

    return (
        <header
            data-slot="diff-header"
            {...style.attrs(styles.header, !control.isCollapsed() && styles.divided)}
        >
            <Button
                isIconOnly
                variant="ghost"
                size="sm"
                aria-expanded={control.isCollapsed() ? "false" : "true"}
                aria-label={locale.render(
                    control.isCollapsed() ? t`Show the changes` : t`Hide the changes`,
                )}
                onClick={() => control.setCollapsed((is) => !is)}
            >
                <Icon icon={control.isCollapsed() ? caretRight : caretDown} />
            </Button>
            <span title={properties.path} {...style.attrs(text.xs, styles.path)}>
                <bdi>
                    <Show when={previous()}>
                        {(moved) => <span {...style.attrs(styles.directory)}>{moved()} → </span>}
                    </Show>
                    <span {...style.attrs(styles.directory)}>
                        {properties.path.slice(0, cut())}
                    </span>
                    <span {...style.attrs(styles.file)}>{properties.path.slice(cut())}</span>
                </bdi>
            </span>
            <CopyButton
                value={properties.path}
                label={locale.render(t`Copy the path`)}
                isIconOnly
                size="sm"
            />
            <Show when={status()}>
                {(said) => <Badge variant="outline">{locale.render(said())}</Badge>}
            </Show>
            <span {...style.attrs(styles.grow)} />
            <span dir="ltr" {...style.attrs(text.xs, styles.counts)}>
                <Show when={added() > 0}>
                    <span {...style.attrs(styles.addedCount)}>+{locale.number(added())}</span>
                </Show>
                <Show when={removed() > 0}>
                    <span {...style.attrs(styles.removedCount)}>−{locale.number(removed())}</span>
                </Show>
                <span aria-hidden="true" {...style.attrs(styles.squares)}>
                    <For each={squares()}>
                        {(square) => (
                            <span
                                {...style.attrs(
                                    styles.square,
                                    square === "added" && styles.addedSquare,
                                    square === "removed" && styles.removedSquare,
                                )}
                            />
                        )}
                    </For>
                </span>
            </span>
            {diff.children}
        </header>
    );
}

/** Render the lines of a diff, block by block as they come near the screen. */
function DiffContent(): JSX.Element {
    // say so where nothing changed
    const { control, properties: diff, isSplit, isWrapped, canSelect } = useDiff();
    const locale = useLocale();
    const digits = (): number => {
        // leave room for two digits at least, so short files look alike
        const { model } = control;
        const lines = Math.max(model().before.lines, model().after.lines);

        return Math.max(2, String(lines).length);
    };
    let isDragging = false;
    const follow = (target: EventTarget | null): DiffPlace | undefined => {
        // go to the line of the cell, or the first of the row that has the focus
        const first =
            target instanceof HTMLTableRowElement ? target.querySelector("td[data-line]") : null;
        const place = placeOf(first ?? target);
        if (place !== undefined) {
            control.setActive(place);
        }

        return place;
    };
    const reason = () => {
        // say what happened to a file whose lines did not change
        const { patch } = diff;

        return patch?.isBinary === true
            ? t.context("option")`Binary file`
            : patch?.status === "renamed"
              ? t`Moved without changes`
              : t.context("option")`Nothing changed`;
    };
    const isEmpty = (): boolean => control.model().difference.changes.length === 0;

    return (
        <Show
            when={!isEmpty()}
            fallback={<p {...style.attrs(styles.empty)}>{locale.render(reason())}</p>}
        >
            <div
                dir="ltr"
                onFocusIn={(event) => follow(event.target)}
                onPointerOver={(event) => {
                    // take the lines a drag passes, and follow the pointer
                    const place = follow(event.target);
                    const isNumber =
                        event.target instanceof Element &&
                        event.target.closest("[data-line-number]") !== null;
                    if (isDragging && event.buttons === 1 && isNumber && place !== undefined) {
                        control.extend(place.side, place.line);
                    }
                }}
                onPointerDown={(event) => {
                    // select a line by its number, or up to it with shift
                    const place = placeOf(event.target);
                    const isNumber =
                        event.target instanceof Element &&
                        event.target.closest("[data-line-number]") !== null;
                    isDragging = false;
                    if (!isNumber || place === undefined || !canSelect() || event.button !== 0) {
                        return;
                    }
                    event.preventDefault();
                    isDragging = true;
                    const { side, line } = place;
                    if (event.shiftKey) {
                        control.extend(side, line);
                    } else {
                        control.start(side, line);
                    }
                }}
                onPointerLeave={(event) => {
                    // stay where the focus is
                    const focused = event.currentTarget.querySelector("tr:focus-within");
                    if (focused === null && event.pointerType === "mouse") {
                        control.setActive(undefined);
                    }
                }}
                {...style.attrs(styles.frame)}
            >
                <div
                    role="group"
                    tabindex={0}
                    aria-label={locale.render(t.context("title")`Lines`)}
                    data-slot="diff-content"
                    onScroll={() => {
                        control.setActive(undefined);
                    }}
                    onKeyDown={(event) => {
                        // leave the selected lines
                        const isSelected = canSelect() && control.selection() !== undefined;
                        if (event.key === "Escape" && isSelected) {
                            event.stopPropagation();
                            control.select(undefined);
                        }
                    }}
                    {...style.attrs(state.focusInset, styles.body)}
                >
                    <table
                        {...style.attributes(
                            [
                                styles.table,
                                isSplit() ? styles.split : !isWrapped() && styles.scrolling,
                            ],
                            { "--diff-digits": String(digits()) },
                        )}
                    >
                        <Show when={isSplit()}>
                            <colgroup>
                                <col {...style.attrs(styles.numberColumn)} />
                                <col />
                                <col {...style.attrs(styles.numberColumn)} />
                                <col />
                            </colgroup>
                        </Show>
                        <For each={control.blocks()} keyed={false}>
                            {(block, place) => <DiffBlock rows={block()} place={place} />}
                        </For>
                    </table>
                </div>
                <DiffTools />
            </div>
        </Show>
    );
}

/** Render a block of rows, or the room they take while they are far from the screen. */
function DiffBlock(properties: {
    /** The rows. */
    readonly rows: readonly (DiffRow | DiffPair)[];
    /** Its place among the blocks. */
    readonly place: number;
}): JSX.Element {
    // render it once it comes near, and leave it there
    const { control, isSplit } = useDiff();
    const place = untrack(() => properties.place);
    let body: HTMLTableSectionElement | undefined;
    onNear(
        () => body,
        () => control.show(place),
    );

    return (
        <tbody ref={(element) => (body = element)} data-slot="diff-block">
            <Show
                when={control.isShown(place)}
                fallback={
                    <tr>
                        <td
                            colspan={isSplit() ? 4 : 3}
                            style={{
                                height: `${String(properties.rows.length * LINE_HEIGHT)}em`,
                            }}
                        />
                    </tr>
                }
            >
                <For each={properties.rows}>
                    {(row) =>
                        row.kind === "fold" ? (
                            <DiffFoldRow fold={row.fold} />
                        ) : row.kind === "line" ? (
                            <DiffLineRow line={row.line} />
                        ) : (
                            <DiffPairRow old={row.old} next={row.next} />
                        )
                    }
                </For>
            </Show>
        </tbody>
    );
}

/** Render a line in one column with its numbers in both texts. */
function DiffLineRow(properties: {
    /** The line. */
    readonly line: DiffLine;
}): JSX.Element {
    const { control, isWrapped } = useDiff();
    const line = untrack(() => properties.line);

    return (
        <>
            <tr {...rowAttributes(line)} {...style.attrs(style.defaultMarker())}>
                <DiffNumber line={line} side="before" hasBar />
                <DiffNumber line={line} side="after" />
                <td
                    {...cellAttributes(line, line.after === undefined ? "before" : "after")}
                    {...style.attrs(styles.code, toneOf(line), isWrapped() && styles.wrapped)}
                >
                    <DiffKind line={line} />
                    <DiffCode control={control} line={line} />
                </td>
            </tr>
            <DiffAnnotations line={line} columns={3} />
        </>
    );
}

/** Render the lines that stand beside each other. */
function DiffPairRow(properties: {
    /** The line of the old text, absent where it has none. */
    readonly old: DiffLine | undefined;
    /** The line of the new text, absent where it has none. */
    readonly next: DiffLine | undefined;
}): JSX.Element {
    const { old, next } = untrack(() => ({ old: properties.old, next: properties.next }));
    const shown = next ?? old;

    return (
        <>
            <tr
                {...(shown === undefined ? {} : rowAttributes(shown))}
                {...style.attrs(style.defaultMarker())}
            >
                <DiffHalf line={old} side="before" isFirst />
                <DiffHalf line={next} side="after" isFirst={false} />
            </tr>
            <Show when={old !== undefined && old !== next ? old : undefined}>
                {(line) => <DiffAnnotations line={line()} columns={4} />}
            </Show>
            <Show when={next}>{(line) => <DiffAnnotations line={line()} columns={4} />}</Show>
        </>
    );
}

/** Render one side of lines beside each other, empty where its text has none. */
function DiffHalf(properties: {
    /** The line, absent where the text has none. */
    readonly line: DiffLine | undefined;
    /** Whose line. */
    readonly side: DiffSide;
    /** Whether it is the first of both. */
    readonly isFirst: boolean;
}): JSX.Element {
    const { control } = useDiff();

    return (
        <Show
            when={properties.line}
            fallback={
                <>
                    <td
                        {...style.attrs(
                            styles.number,
                            styles.missing,
                            !properties.isFirst && styles.half,
                        )}
                    />
                    <td {...style.attrs(styles.code, styles.missing)} />
                </>
            }
        >
            {(line) => (
                <>
                    <DiffNumber
                        line={line()}
                        side={properties.side}
                        hasBar={line().kind !== "same"}
                        isHalf={!properties.isFirst}
                    />
                    <td
                        {...cellAttributes(line(), properties.side)}
                        {...style.attrs(styles.code, toneOf(line()), styles.wrapped)}
                    >
                        <DiffKind line={line()} />
                        <DiffCode control={control} line={line()} />
                    </td>
                </>
            )}
        </Show>
    );
}

/** Render a line's number in a text, which a press selects the line by. */
function DiffNumber(properties: {
    /** The line. */
    readonly line: DiffLine;
    /** Whose number. */
    readonly side: DiffSide;
    /** Whether it has the bar of a change. */
    readonly hasBar?: boolean;
    /** Whether a line divides it from the other text. */
    readonly isHalf?: boolean;
}): JSX.Element {
    // read what never changes of it once
    const { control, canSelect } = useDiff();
    const { line, side, hasBar, isHalf } = untrack(() => ({ ...properties }));
    const place = line[side];
    const number = place === undefined ? "" : String(place + 1);
    const tone =
        line.kind === "added"
            ? styles.addedNumber
            : line.kind === "removed"
              ? styles.removedNumber
              : null;
    const isSelected = (): boolean => control.isSelected(line, side);

    return (
        <td
            data-line-number={number}
            {...cellAttributes(line, side)}
            data-selected={isSelected() ? "" : undefined}
            {...style.attrs(
                styles.number,
                side === "before" ? styles.firstNumber : styles.secondNumber,
                isHalf === true && styles.half,
                canSelect() && place !== undefined && styles.pickable,
                tone,
                isSelected() && styles.selected,
            )}
        >
            {hasBar === true && line.kind !== "same" ? (
                <span
                    aria-hidden="true"
                    {...style.attrs(
                        styles.bar,
                        line.kind === "added" ? styles.addedBar : styles.removedBar,
                    )}
                />
            ) : null}
            {number}
        </td>
    );
}

/** Read the attributes of a cell, which the line the person is at is found by. */
function cellAttributes(
    line: DiffLine,
    side: DiffSide,
): { readonly "data-side"?: DiffSide; readonly "data-line"?: string } {
    const place = line[side];

    return place === undefined ? {} : { "data-side": side, "data-line": String(place) };
}

/** Render whether a line was added or removed, to assistive technology. */
function DiffKind(properties: {
    /** The line. */
    readonly line: DiffLine;
}): JSX.Element {
    // say nothing of an unchanged line
    const locale = useLocale();
    const { kind } = untrack(() => properties.line);

    return kind === "same" ? null : (
        <span {...style.attrs(visuallyHiddenStyle())}>
            {locale.render(
                kind === "added" ? t.context("option")`added` : t.context("option")`removed`,
            )}
        </span>
    );
}

/** Render what a person does with the line and the change they are at. */
function DiffTools(): JSX.Element {
    // place them at the line, and at the first line of its change
    const { control, properties: diff } = useDiff();
    const locale = useLocale();
    const change = (): HTMLElement | undefined => {
        const row = control.active()?.row;
        const place = row?.dataset["change"];

        return place === undefined
            ? undefined
            : (row
                  ?.closest("table")
                  ?.querySelector<HTMLElement>(`[data-change-start][data-change="${place}"]`) ??
                  row);
    };

    return (
        <>
            <Show when={diff.onComment !== undefined ? control.active() : undefined}>
                {(place) => (
                    <button
                        type="button"
                        tabindex={-1}
                        aria-label={locale.render(t`Comment on line ${String(place().line + 1)}`)}
                        onClick={() => diff.onComment?.(place().side, place().line)}
                        {...style.attributes([state.focus, styles.comment], {
                            top: topOf(place().row),
                            left: startOf(place()),
                        })}
                    >
                        <Icon icon={plus} size="0.8em" />
                    </button>
                )}
            </Show>
            <Show
                when={
                    diff.onAccept !== undefined || diff.onReject !== undefined
                        ? change()
                        : undefined
                }
            >
                {(row) => (
                    <div
                        data-slot="diff-change-actions"
                        {...style.attributes([styles.changeActions], { top: topOf(row()) })}
                    >
                        <Show when={diff.onReject}>
                            <ActionButton
                                action={rejectChange}
                                element={row()}
                                variant="ghost"
                                isIconOnly
                                size="sm"
                                icon={xIcon}
                            />
                        </Show>
                        <Show when={diff.onAccept}>
                            <ActionButton
                                action={acceptChange}
                                element={row()}
                                variant="ghost"
                                isIconOnly
                                size="sm"
                                icon={check}
                            />
                        </Show>
                    </div>
                )}
            </Show>
        </>
    );
}

/** Render what stands under a line. */
function DiffAnnotations(properties: {
    /** The line. */
    readonly line: DiffLine;
    /** The columns there are. */
    readonly columns: number;
}): JSX.Element {
    // read them by the line of either text
    const { annotations } = useDiff();
    const { before, after } = untrack(() => properties.line);
    const found = (): readonly DiffAnnotation[] => {
        // look nothing up where there are none
        const byLine = annotations();
        if (byLine.size === 0) {
            return [];
        }

        return [
            ...(byLine.get(`before ${String(before)}`) ?? []),
            ...(byLine.get(`after ${String(after)}`) ?? []),
        ];
    };

    return (
        <For each={found()}>
            {(annotation) => (
                <tr data-slot="diff-annotation">
                    <td colspan={properties.columns} {...style.attrs(text.sm, styles.annotation)}>
                        <div dir="auto" {...style.attrs(styles.annotationBody)}>
                            {annotation.content()}
                        </div>
                    </td>
                </tr>
            )}
        </For>
    );
}

/** Render lines folded away, with how many there are and what reveals them. */
function DiffFoldRow(properties: {
    /** The fold. */
    readonly fold: DiffFold;
}): JSX.Element {
    // offer some lines of a long fold
    const { control, isSplit } = useDiff();
    const locale = useLocale();
    const fold = untrack(() => properties.fold);
    const count = fold.before.end - fold.before.start;
    const isLong = count > REVEALED_LINES;

    return (
        <tr data-slot="diff-fold">
            <td colspan={isSplit() ? 4 : 3} {...style.attrs(text.xs, styles.fold)}>
                <div {...style.attrs(styles.foldLine)}>
                    <Show when={!fold.isMissing}>
                        <Show when={isLong && fold.position !== "first"}>
                            <Button
                                isIconOnly
                                variant="ghost"
                                size="sm"
                                aria-label={locale.render(t`Show 20 lines below the change above`)}
                                onClick={() => control.reveal(fold, "above")}
                            >
                                <Icon icon={arrowDown} />
                            </Button>
                        </Show>
                        <Show when={isLong && fold.position !== "last"}>
                            <Button
                                isIconOnly
                                variant="ghost"
                                size="sm"
                                aria-label={locale.render(t`Show 20 lines above the next change`)}
                                onClick={() => control.reveal(fold, "below")}
                            >
                                <Icon icon={arrowUp} />
                            </Button>
                        </Show>
                        <Button
                            isIconOnly
                            variant="ghost"
                            size="sm"
                            aria-label={locale.render(t`Show every line`)}
                            onClick={() => control.reveal(fold, "all")}
                        >
                            <Icon icon={arrowsOut} />
                        </Button>
                    </Show>
                    <span {...style.attrs(styles.count)}>
                        {locale.render(
                            t`${plural(argument("count"), { one: "# unchanged line", other: "# unchanged lines" })}`,
                            { count },
                        )}
                    </span>
                    <Show when={fold.heading}>
                        {(heading) => (
                            <span title={heading()} {...style.attrs(styles.heading)}>
                                {heading()}
                            </span>
                        )}
                    </Show>
                </div>
            </td>
        </tr>
    );
}

/** Read the attributes of a line's row, which its change is found by. */
function rowAttributes(line: DiffLine): JSX.HTMLAttributes<HTMLTableRowElement> {
    return {
        "data-kind": line.kind,
        ...(line.place === undefined ? {} : { "data-change": String(line.place) }),
        ...(isFirstOf(line) ? { "data-change-start": "", tabindex: -1 } : {}),
    };
}

/** Read the line of the cell an event is in, absent outside one. */
function placeOf(target: EventTarget | null): DiffPlace | undefined {
    // read the cell's line
    const cell = target instanceof Element ? target.closest<HTMLElement>("td[data-line]") : null;
    const row = cell?.closest("tr") ?? undefined;
    const { side, line } = cell?.dataset ?? {};

    return row === undefined || (side !== "before" && side !== "after")
        ? undefined
        : { row, side, line: Number(line) };
}

/** Read how far down the lines a row is. */
function topOf(row: HTMLElement): string {
    const lines = row.closest("[data-slot=diff-content]")?.getBoundingClientRect().top ?? 0;

    return `${String(row.getBoundingClientRect().top - lines)}px`;
}

/** Read how far across the lines the number beside a line's code ends. */
function startOf({ row, side }: DiffPlace): string {
    // measure the number against the lines
    const code = row.querySelector(`[data-side="${side}"]:not([data-line-number])`);
    const number = code?.previousElementSibling?.getBoundingClientRect().right ?? 0;
    const lines = row.closest("[data-slot=diff-content]")?.getBoundingClientRect().left ?? 0;

    return `${String(number - lines)}px`;
}

/** Report whether a line is the first of its change. */
function isFirstOf({ change, before, after }: DiffLine): boolean {
    return (
        change !== undefined &&
        (change.before.start < change.before.end
            ? before === change.before.start
            : after === change.after.start)
    );
}

/** Read the background of a line. */
function toneOf(line: DiffLine): style.Styles {
    return line.kind === "added" ? styles.added : line.kind === "removed" ? styles.removed : null;
}
