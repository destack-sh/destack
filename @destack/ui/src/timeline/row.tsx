import { state } from "@destack/theme/state";
import { Icon } from "@destack/ui/icon";
import dotsSixVertical from "@destack/icon/phosphor/dots-six-vertical";
import { nth } from "@destack/schema";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type Accessor, For, type JSX, omit, Show, untrack, useLocale } from "@destack/view";
import { Draggable } from "../drag/index.ts";
import { DisclosureCaret, indent, indicator } from "../disclosure/index.ts";
import { Press } from "../interaction/index.ts";
import { contentOr, mergeProperties, renderPart } from "../part/index.ts";
import {
    TimelineRowContext,
    useTimeline,
    useTimelineRow,
    type TimelineControl,
    type TimelineElementProperties,
    type TimelinePartProperties,
} from "./control.ts";
import type { TimelineNode } from "./tree.ts";
import { ROW_TYPE, ROWS } from "./drag.ts";
import { places } from "./places.ts";
import { TimelineTrack } from "./track.tsx";
import { rowMarker } from "./marker.stylex.ts";

/** The elements of a label that take a press themselves. */
const CONTROLS = "button, a, input, [data-slot=timeline-row-grip]";

/** The label a row's grip stands in. */
const LABEL_SLOT = "[data-slot=timeline-row-label]";

/** The styles of a timeline's rows. */
const styles = style.create({
    row: {
        position: "absolute",
        insetInline: 0,
        display: "grid",
    },
    label: {
        position: "relative",
        display: "flex",
        alignItems: "center",
        gap: space[1],
        minWidth: 0,
        paddingInlineEnd: space[2],
        borderInlineEndWidth: stroke.border,
        borderInlineEndStyle: "solid",
        borderInlineEndColor: color.border,
        overflow: "hidden",
    },
    words: {
        minWidth: 0,
        whiteSpace: "nowrap",
        overflow: "hidden",
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
    },
    cell: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
        minWidth: 0,
        paddingInline: space[2],
        overflow: "hidden",
        borderInlineEndWidth: stroke.border,
        borderInlineEndStyle: "solid",
        borderInlineEndColor: color.border,
        whiteSpace: "nowrap",
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
    },
    toggle: {
        display: "inline-flex",
        flexShrink: 0,
        justifyContent: "center",
        width: space[4],
        color: color.mutedForeground,
    },
    grip: {
        position: "absolute",
        insetBlock: 0,
        insetInlineStart: 0,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        width: space[4],
        color: color.mutedForeground,
        cursor: { default: "grab", ":active": "grabbing" },
        opacity: {
            default: 0,
            [style.when.ancestor(":hover", rowMarker)]: 1,
            [style.when.ancestor(":focus-within", rowMarker)]: 1,
        },
    },
    rowDrop: {
        position: "absolute",
        insetInlineEnd: 0,
        zIndex: 4,
        height: stroke.ring,
        marginBlockStart: `calc(${stroke.ring} / -2)`,
        backgroundColor: swatch.accentMark,
        pointerEvents: "none",
    },
});

/** The properties of a timeline's rows. */
export interface TimelineRowListProperties {
    /** Render a row, a label and a track by default. */
    readonly children?: (node: Accessor<TimelineNode>) => JSX.Element;
}

/** Render the rows in view, each from the function given or as a label and a track. */
export function TimelineRowList(properties: TimelineRowListProperties): JSX.Element {
    const control = useTimeline();

    return (
        <>
            {/* The gap a dragged row lands in */}
            <Show when={control.drag.rowGap()}>
                {(gap) => (
                    <div
                        data-slot="timeline-row-drop"
                        data-depth={gap().depth}
                        aria-hidden="true"
                        {...style.attrs(
                            styles.rowDrop,
                            places.top(control.rowTop(gap().index)),
                            places.inset(space[4], gap().depth),
                        )}
                    />
                )}
            </Show>
            <For each={control.virtualizer.items()} keyed={(virtual) => virtual.key}>
                {(virtual) => {
                    // read the row the virtual item stands for, which the virtualizer counts
                    const node = () => nth(control.tree.rows(), virtual().index);

                    return (
                        <TimelineRowContext value={{ node, index: () => virtual().index }}>
                            {properties.children === undefined ? (
                                <TimelineRow />
                            ) : (
                                properties.children(node)
                            )}
                        </TimelineRowContext>
                    );
                }}
            </For>
        </>
    );
}

/** Render a row at its place: its label and its track by default. */
export function TimelineRow(properties: TimelinePartProperties): JSX.Element {
    // focus the row as the timeline's tab stop
    const control = useTimeline();
    const row = useTimelineRow();
    const key = untrack(() => row.node().item.key);
    const rest = omit(properties, "children");
    let element: HTMLElement | undefined;
    control.focus.bind(
        () => key,
        () => element,
    );
    const isFolded = () => control.tree.folded().has(key);
    const attributes = {
        ref: (created: HTMLDivElement) => (element = created),
        onFocus: () => control.focus.focusIn(key),
    };

    return renderPart(
        "div",
        "timeline-row",
        rest,
        () => [
            rowMarker,
            text.sm,
            state.focusInset,
            state.hover,
            styles.row,
            places.columns(control.template()),
            places.row(
                control.rowTop(row.index()),
                control.rowTop(row.index() + 1) - control.rowTop(row.index()),
            ),
            control.selection.isSelected(key) && state.selected,
        ],
        mergeProperties(
            {
                role: "row" as const,
                "data-key": key,
                get "aria-rowindex"() {
                    return row.index() + 1;
                },
                get "aria-selected"() {
                    return control.selection.isSelected(key) ? "true" : "false";
                },
                get "aria-level"() {
                    return row.node().depth + 1;
                },
                get "aria-expanded"() {
                    return row.node().hasChildren ? (isFolded() ? "false" : "true") : undefined;
                },
                get tabindex() {
                    return control.focus.isActive(key) ? 0 : -1;
                },
                get children() {
                    return contentOr(properties, () => (
                        <>
                            <TimelineRowLabel />
                            <TimelineTrack />
                        </>
                    ));
                },
            },
            attributes,
        ),
    );
}

/** Render a row's label, indented by its depth: its grip, guide, toggle and words by default. */
export function TimelineRowLabel(properties: TimelinePartProperties): JSX.Element {
    // leave room for the grip of rows that move
    const control = useTimeline();
    const { node } = useTimelineRow();
    const gutter = untrack(() => gutterOf(control));
    const rest = omit(properties, "children");

    // select or open the item on a press beside the buttons in the label
    const press = new Press({
        onPress: (event) => {
            const { target } = event;
            if (target instanceof Element && target.closest(CONTROLS) === null) {
                control.press(
                    untrack(() => node().item.key),
                    event,
                );
            }
        },
        keys: [],
        shouldCancelOnMove: true,
    });

    return renderPart(
        "div",
        "timeline-row-label",
        rest,
        () => [styles.label, indent.by(gutter, node().depth)],
        mergeProperties(
            {
                role: "gridcell" as const,
                get children() {
                    return contentOr(properties, () => (
                        <>
                            <TimelineRowGrip />
                            <TimelineRowGuide />
                            <TimelineRowToggle />
                            <span {...style.attrs(styles.words)}>{node().item.label}</span>
                        </>
                    ));
                },
            },
            press.attributes(),
        ),
    );
}

/** Read the room before a timeline's labels, which a grip stands in. */
function gutterOf(control: TimelineControl): string {
    return control.options.onRowMove === undefined ? space[2] : space[4];
}

/** Render the grip a row moves by, where its timeline takes moved rows. */
export function TimelineRowGrip(properties: TimelineElementProperties): JSX.Element {
    // move the row with its label
    const control = useTimeline();
    const locale = useLocale();
    const { node } = useTimelineRow();
    const key = untrack(() => node().item.key);
    let element: HTMLElement | undefined;
    if (untrack(() => control.options.onRowMove === undefined)) {
        return null;
    }
    const grip = new Draggable(
        {
            id: JSON.stringify([ROWS, key]),
            item: key,
            type: ROW_TYPE,
            kind: "move",
            label: () => node().item.label,
            element: () => element?.closest<HTMLElement>(LABEL_SLOT) ?? undefined,
            home: () => ({
                kind: "list",
                droppable: ROWS,
                index: control.tree.rows().findIndex((row) => row.item.key === key),
                depth: node().depth,
            }),
        },
        control.drag.manager,
    ).attributes([" "]);

    return renderPart(
        "span",
        "timeline-row-grip",
        properties,
        [text.xs, styles.grip],
        mergeProperties(
            mergeProperties({ ref: (created: HTMLElement) => (element = created) }, grip),
            {
                tabindex: -1,
                get "aria-label"() {
                    return locale.render(t`Move ${node().item.label}`);
                },
                get children() {
                    return <Icon icon={dotsSixVertical} />;
                },
            },
        ),
    );
}

/** Render the lines of the rows a row is under. */
export function TimelineRowGuide(properties: TimelineElementProperties): JSX.Element {
    // start under the toggle of the outermost row
    const control = useTimeline();
    const { node } = useTimelineRow();
    const gutter = untrack(() => gutterOf(control));

    return (
        <Show when={node().depth > 0}>
            {renderPart(
                "span",
                "timeline-row-guide",
                properties,
                () => [indent.guide, indent.guideAt(gutter, node().depth)],
                { "aria-hidden": "true" as const },
            )}
        </Show>
    );
}

/** Render the button that folds the rows under a row, or the room of one where rows nest. */
export function TimelineRowToggle(
    properties: TimelineElementProperties<HTMLButtonElement>,
): JSX.Element {
    // turn with the row's fold
    const control = useTimeline();
    const locale = useLocale();
    const { node } = useTimelineRow();
    const key = untrack(() => node().item.key);
    const isFolded = (): boolean => control.tree.folded().has(key);

    return (
        <Show when={control.tree.isNested()}>
            <Show when={node().hasChildren} fallback={<span {...style.attrs(styles.toggle)} />}>
                {renderPart(
                    "button",
                    "timeline-row-toggle",
                    properties,
                    () => [styles.toggle, indicator.base, !isFolded() && indicator.open],
                    {
                        type: "button" as const,
                        tabindex: -1,
                        onClick: () => control.tree.toggle(key),
                        get "aria-label"() {
                            return locale.render(isFolded() ? t`Expand` : t`Collapse`);
                        },
                        get children() {
                            return <DisclosureCaret />;
                        },
                    },
                )}
            </Show>
        </Show>
    );
}

/** Render a row's cell of a column beside its label, in the order of the header's columns. */
export function TimelineCell(properties: TimelinePartProperties): JSX.Element {
    const rest = properties;

    return renderPart("div", "timeline-cell", rest, [text.xs, styles.cell], {
        role: "gridcell" as const,
    });
}
