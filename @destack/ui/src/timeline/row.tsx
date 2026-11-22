import { state } from "@destack/theme/state";
import { Icon } from "@destack/ui/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import dotsSixVertical from "@destack/icon/phosphor/dots-six-vertical";
import caretRight from "@destack/icon/phosphor/caret-right";
import { nth } from "@destack/schema";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, motion, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type Accessor, For, type JSX, omit, Show, untrack, useLocale } from "@destack/view";
import { Draggable } from "../drag/index.ts";
import { mergeProperties } from "../part/index.ts";
import {
    TimelineRowContext,
    useTimeline,
    useTimelineRow,
    type TimelinePartProperties,
} from "./control.ts";
import type { TimelineNode } from "./tree.ts";
import { ROW_TYPE, ROWS } from "./drag.ts";
import { places } from "./places.ts";
import { TimelineBar } from "./bar.tsx";
import { rowMarker } from "./marker.stylex.ts";

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
        whiteSpace: "nowrap",
        overflow: "hidden",
        textOverflow: "ellipsis",
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
    },
    disclosure: {
        display: "inline-flex",
        justifyContent: "center",
        flexShrink: 0,
        width: space[4],
        color: color.mutedForeground,
        transitionProperty: "transform",
        transitionDuration: motion.durationShort,
    },
    unfolded: {
        transform: "rotate(90deg)",
    },
    unfoldedBack: {
        transform: "rotate(-90deg)",
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
export interface TimelineRowsProperties {
    /** Render a row, a label and a bar by default. */
    readonly children?: (node: Accessor<TimelineNode>) => JSX.Element;
}

/** Render the rows in view, each from the function given or as a label and a bar. */
export function TimelineRows(properties: TimelineRowsProperties): JSX.Element {
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

/** Render a row at its place: its label and its bar by default. */
export function TimelineRow(properties: TimelinePartProperties): JSX.Element {
    // focus the row as the timeline's tab stop
    const control = useTimeline();
    const row = useTimelineRow();
    const key = untrack(() => row.node().item.key);
    const rest = omit(properties, "children", "xstyle", "style");
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

    return (
        <div
            role="row"
            data-slot="timeline-row"
            data-key={key}
            aria-rowindex={row.index() + 1}
            aria-selected={control.selection.isSelected(key) ? "true" : "false"}
            aria-level={row.node().depth + 1}
            aria-expanded={row.node().hasChildren ? (isFolded() ? "false" : "true") : undefined}
            tabindex={control.focus.isActive(key) ? 0 : -1}
            {...mergeProperties(attributes, rest)}
            {...style.attributes(
                [
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
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            {properties.children ?? (
                <>
                    <TimelineRowLabel />
                    <TimelineBar />
                </>
            )}
        </div>
    );
}

/** Render a row's label, indented by its depth, with a fold where items belong under it. */
export function TimelineRowLabel(properties: TimelinePartProperties): JSX.Element {
    // read the row and its fold
    const control = useTimeline();
    const locale = useLocale();
    const node = useTimelineRow().node;
    const key = untrack(() => node().item.key);
    const rest = omit(properties, "children", "xstyle", "style");
    const isFolded = () => control.tree.folded().has(key);
    const isRightToLeft = () => locale.direction === "rtl";
    const isTurned = () => !isFolded() && !isRightToLeft();
    const isTurnedBack = () => !isFolded() && isRightToLeft();

    // move the row by its grip when the timeline takes moved rows
    let element: HTMLElement | undefined;
    const grip = untrack(() => control.options.onRowMove !== undefined)
        ? new Draggable(
              {
                  id: JSON.stringify([ROWS, key]),
                  item: key,
                  type: ROW_TYPE,
                  kind: "move",
                  label: () => node().item.label,
                  element: () => element,
                  home: () => ({
                      kind: "list",
                      droppable: ROWS,
                      index: control.tree.rows().findIndex((row) => row.item.key === key),
                      depth: node().depth,
                  }),
              },
              control.drag.manager,
          ).attributes([" "])
        : undefined;

    return (
        <div
            role="gridcell"
            data-slot="timeline-row-label"
            ref={(created) => (element = created)}
            {...rest}
            {...style.attributes(
                [
                    styles.label,
                    places.indent(grip === undefined ? space[2] : space[4], node().depth),
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            {/* The grip a row moves by */}
            <Show when={grip}>
                {(attributes) => (
                    <span
                        tabindex={-1}
                        data-slot="timeline-row-grip"
                        aria-label={locale.render(t`Move ${node().item.label}`)}
                        {...attributes()}
                        {...style.attrs(text.xs, styles.grip)}
                    >
                        <Icon icon={dotsSixVertical} />
                    </span>
                )}
            </Show>

            <Show when={control.tree.isNested()}>
                <Show
                    when={node().hasChildren}
                    fallback={<span {...style.attrs(styles.disclosure)} />}
                >
                    <button
                        type="button"
                        tabindex={-1}
                        aria-label={locale.render(isFolded() ? t`Expand` : t`Collapse`)}
                        onClick={() => control.tree.toggle(key)}
                        {...style.attrs(
                            styles.disclosure,
                            isTurned() && styles.unfolded,
                            isTurnedBack() && styles.unfoldedBack,
                        )}
                    >
                        <Icon icon={isRightToLeft() ? caretLeft : caretRight} />
                    </button>
                </Show>
            </Show>
            {properties.children ?? node().item.label}
        </div>
    );
}

/** Render a row's cell of a column beside its label, in the order of the header's columns. */
export function TimelineCell(properties: TimelinePartProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            role="gridcell"
            data-slot="timeline-cell"
            {...rest}
            {...style.attributes([text.xs, styles.cell, properties.xstyle], properties.style)}
        />
    );
}
