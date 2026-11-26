import { field } from "@destack/object";
import { type Axis, ORIENTATION } from "../position/index.ts";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, space } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

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
export type DataListOrientation = Axis;

/** The properties of an element of a data list, the native element's attributes included. */
export type DataListElementProperties<Target extends HTMLElement = HTMLElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> &
    ElementPartProperties;

/** The properties people change in a data list's controls and inspector. */
const DATA_LIST_PROPERTIES = {
    orientation: ORIENTATION.described(
        t`Whether each label sits beside its value or above it.`,
    ).default("horizontal"),
};

/** The properties of a data list besides its declared ones: its native attributes. */
export interface DataListAttributes extends DataListElementProperties<HTMLDListElement> {}

/** Render a list of labelled values, such as an error's code and request id, after Radix's. */
export const DataList = defineComponent({
    name: "dataList",
    properties: DATA_LIST_PROPERTIES,
    render(
        properties: RenderProperties<DataListAttributes, typeof DATA_LIST_PROPERTIES>,
    ): JSX.Element {
        const rest = omit(properties, "orientation");

        return renderPart(
            "dl",
            "data-list",
            rest,
            () => [text.sm, styles.list, styles[properties.orientation]],
            {
                get "data-orientation"() {
                    return properties.orientation;
                },
            },
        );
    },
});

/** The properties of a data list, its native attributes included. */
export type DataListProperties = ComponentProperties<typeof DataList>;

/** The properties people change in a data list entry's controls and inspector. */
const DATA_LIST_ITEM_PROPERTIES = {
    align: field
        .enum({
            start: { title: t.context("option")`Start` },
            center: { title: t.context("option")`Center` },
            end: { title: t.context("option")`End` },
            baseline: { title: t.context("option")`Baseline` },
            stretch: { title: t.context("option")`Stretch` },
        })
        .labeled(t.context("label")`Align`)
        .described(t`How the label lines up with its value.`)
        .default("baseline"),
};

/** The properties of a data list entry besides its declared ones: its native attributes. */
export interface DataListItemAttributes extends DataListElementProperties<HTMLDivElement> {}

/** Render one label and its value of a data list, laid along the list's orientation. */
export const DataListItem = defineComponent({
    name: "dataListItem",
    properties: DATA_LIST_ITEM_PROPERTIES,
    render(
        properties: RenderProperties<DataListItemAttributes, typeof DATA_LIST_ITEM_PROPERTIES>,
    ): JSX.Element {
        return renderPart("div", "data-list-item", omit(properties, "align"), () => [
            styles.item,
            alignments[properties.align],
        ]);
    },
});

/** The properties of a data list entry, its native attributes included. */
export type DataListItemProperties = ComponentProperties<typeof DataListItem>;

/** Render the label of a data list's item. */
export function DataListLabel(properties: DataListElementProperties): JSX.Element {
    return renderPart("dt", "data-list-label", properties, styles.label);
}

/** Render the value of a data list's item. */
export function DataListValue(properties: DataListElementProperties): JSX.Element {
    return renderPart("dd", "data-list-value", properties, styles.value);
}
