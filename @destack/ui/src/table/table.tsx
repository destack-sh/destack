import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The styles of a table and its elements. */
const styles = style.create({
    container: {
        position: "relative",
        width: "100%",
        overflowX: "auto",
    },
    table: {
        width: "100%",
        borderCollapse: "collapse",
        captionSide: "bottom",
    },
    footer: {
        backgroundColor: `color-mix(in oklab, ${color.muted} 50%, transparent)`,
        fontWeight: weight.medium,
    },
    row: {
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        borderBottomColor: color.border,
        backgroundColor: {
            default: "transparent",
            ":hover": {
                default: null,
                [media.hover]: `color-mix(in oklab, ${color.muted} 50%, transparent)`,
            },
        },
    },
    head: {
        height: size[3],
        paddingInline: space[2],
        textAlign: "start",
        verticalAlign: "middle",
        whiteSpace: "nowrap",
        color: color.foreground,
        fontWeight: weight.medium,
    },
    cell: {
        padding: space[2],
        verticalAlign: "middle",
        whiteSpace: "nowrap",
    },
    caption: {
        marginTop: space[4],
        color: color.mutedForeground,
    },
});

/** The properties of an element of a table, the native element's attributes included. */
export type TableElementProperties<Attributes> = Omit<Attributes, "class"> & ElementPartProperties;

/** Render a native table that scrolls sideways when wider than its container. */
export function Table(
    properties: TableElementProperties<JSX.HTMLAttributes<HTMLTableElement>>,
): JSX.Element {
    return (
        <div data-slot="table-container" {...style.attrs(styles.container)}>
            {renderPart("table", "table", properties, [text.footnote, styles.table])}
        </div>
    );
}

/** Render the header rows of a table. */
export function TableHeader(
    properties: TableElementProperties<JSX.HTMLAttributes<HTMLTableSectionElement>>,
): JSX.Element {
    return renderPart("thead", "table-header", properties, null);
}

/** Render the body rows of a table. */
export function TableBody(
    properties: TableElementProperties<JSX.HTMLAttributes<HTMLTableSectionElement>>,
): JSX.Element {
    return renderPart("tbody", "table-body", properties, null);
}

/** Render the footer rows of a table, such as totals. */
export function TableFooter(
    properties: TableElementProperties<JSX.HTMLAttributes<HTMLTableSectionElement>>,
): JSX.Element {
    return renderPart("tfoot", "table-footer", properties, styles.footer);
}

/** Render a row of a table. */
export function TableRow(
    properties: TableElementProperties<JSX.HTMLAttributes<HTMLTableRowElement>>,
): JSX.Element {
    return renderPart("tr", "table-row", properties, styles.row);
}

/** Render a header cell that names its column or row. */
export function TableHead(
    properties: TableElementProperties<JSX.ThHTMLAttributes<HTMLTableCellElement>>,
): JSX.Element {
    return renderPart("th", "table-head", properties, styles.head);
}

/** Render a data cell. */
export function TableCell(
    properties: TableElementProperties<JSX.TdHTMLAttributes<HTMLTableCellElement>>,
): JSX.Element {
    return renderPart("td", "table-cell", properties, styles.cell);
}

/** Render the caption that names a table. */
export function TableCaption(
    properties: TableElementProperties<JSX.HTMLAttributes<HTMLTableCaptionElement>>,
): JSX.Element {
    return renderPart("caption", "table-caption", properties, styles.caption);
}
