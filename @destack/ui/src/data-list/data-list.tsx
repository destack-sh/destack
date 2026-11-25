import * as style from "@destack/style";
import { color, space } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, merge, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The orientation of a data list that sets none. */
const DEFAULTS: Required<Pick<DataListProperties, "orientation">> = { orientation: "horizontal" };

/** The alignment of a data list's item that sets none. */
const ITEM_DEFAULTS: Required<Pick<DataListItemProperties, "align">> = { align: "baseline" };

/** The selector of an item in a vertical data list. */
const VERTICAL = ":is([data-orientation=vertical] > *)";

/** The styles of a data list and its elements. */
const styles = style.create({
    list: {
        display: "grid",
        // leave free room below the items
        alignContent: "start",
        margin: 0,
        rowGap: space[2],
    },
    horizontal: {
        gridTemplateColumns: "minmax(auto, max-content) 1fr",
        columnGap: space[4],
    },
    vertical: {
        gridTemplateColumns: "1fr",
        rowGap: space[3],
    },
    item: {
        display: "grid",
        gridColumn: "1 / -1",
        gridTemplateColumns: { default: "subgrid", [VERTICAL]: "1fr" },
        rowGap: { default: 0, [VERTICAL]: space[1] },
    },
    label: {
        margin: 0,
        color: color.mutedForeground,
    },
    value: {
        margin: 0,
        minWidth: 0,
        overflowWrap: "anywhere",
        color: color.foreground,
    },
});

/** The alignment of a data list item's label with its value. */
const alignments = style.create({
    start: { alignItems: "start" },
    center: { alignItems: "center" },
    end: { alignItems: "end" },
    baseline: { alignItems: "baseline" },
    stretch: { alignItems: "stretch" },
});

/** The axis a data list sets each label and value along: side by side, or the label above. */
export type DataListOrientation = "horizontal" | "vertical";

/** The properties of an element of a data list, the native element's attributes included. */
export type DataListElementProperties<Target extends HTMLElement = HTMLElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> &
    ElementPartProperties;

/** The properties of a data list, the native element's attributes included. */
export interface DataListProperties extends DataListElementProperties<HTMLDListElement> {
    /** The axis of each label and value, horizontal by default. */
    readonly orientation?: DataListOrientation;
}

/** Render a list of labelled values, such as an error's code and request id, after Radix's. */
export function DataList(properties: DataListProperties): JSX.Element {
    const list = merge(DEFAULTS, properties);
    const rest = omit(list, "orientation");

    return renderPart(
        "dl",
        "data-list",
        rest,
        () => [text.sm, styles.list, styles[list.orientation]],
        {
            get "data-orientation"() {
                return list.orientation;
            },
        },
    );
}

/** The properties of a data list's item, the native element's attributes included. */
export interface DataListItemProperties extends DataListElementProperties<HTMLDivElement> {
    /** The alignment of the label with its value, on their baseline by default. */
    readonly align?: keyof typeof alignments;
}

/** Render one label and its value of a data list, laid along the list's orientation. */
export function DataListItem(properties: DataListItemProperties): JSX.Element {
    const item = merge(ITEM_DEFAULTS, properties);

    return renderPart("div", "data-list-item", omit(item, "align"), () => [
        styles.item,
        alignments[item.align],
    ]);
}

/** Render the label of a data list's item. */
export function DataListLabel(properties: DataListElementProperties): JSX.Element {
    return renderPart("dt", "data-list-label", properties, styles.label);
}

/** Render the value of a data list's item. */
export function DataListValue(properties: DataListElementProperties): JSX.Element {
    return renderPart("dd", "data-list-value", properties, styles.value);
}
