import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, omit } from "@destack/view";

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
export type TableElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** Render a native table that scrolls sideways when wider than its container. */
export function Table(
    properties: TableElementProperties<JSX.HTMLAttributes<HTMLTableElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <div data-slot="table-container" {...style.attrs(styles.container)}>
            <table
                data-slot="table"
                {...rest}
                {...style.attributes(
                    [text.footnote, styles.table, properties.xstyle],
                    properties.style,
                )}
            />
        </div>
    );
}

/** Render the header rows of a table. */
export function TableHeader(
    properties: TableElementProperties<JSX.HTMLAttributes<HTMLTableSectionElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <thead
            data-slot="table-header"
            {...rest}
            {...style.attributes([properties.xstyle], properties.style)}
        />
    );
}

/** Render the body rows of a table. */
export function TableBody(
    properties: TableElementProperties<JSX.HTMLAttributes<HTMLTableSectionElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <tbody
            data-slot="table-body"
            {...rest}
            {...style.attributes([properties.xstyle], properties.style)}
        />
    );
}

/** Render the footer rows of a table, such as totals. */
export function TableFooter(
    properties: TableElementProperties<JSX.HTMLAttributes<HTMLTableSectionElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <tfoot
            data-slot="table-footer"
            {...rest}
            {...style.attributes([styles.footer, properties.xstyle], properties.style)}
        />
    );
}

/** Render a row of a table. */
export function TableRow(
    properties: TableElementProperties<JSX.HTMLAttributes<HTMLTableRowElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <tr
            data-slot="table-row"
            {...rest}
            {...style.attributes([styles.row, properties.xstyle], properties.style)}
        />
    );
}

/** Render a header cell that names its column or row. */
export function TableHead(
    properties: TableElementProperties<JSX.ThHTMLAttributes<HTMLTableCellElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <th
            data-slot="table-head"
            {...rest}
            {...style.attributes([styles.head, properties.xstyle], properties.style)}
        />
    );
}

/** Render a data cell. */
export function TableCell(
    properties: TableElementProperties<JSX.TdHTMLAttributes<HTMLTableCellElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <td
            data-slot="table-cell"
            {...rest}
            {...style.attributes([styles.cell, properties.xstyle], properties.style)}
        />
    );
}

/** Render the caption that names a table. */
export function TableCaption(
    properties: TableElementProperties<JSX.HTMLAttributes<HTMLTableCaptionElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <caption
            data-slot="table-caption"
            {...rest}
            {...style.attributes([styles.caption, properties.xstyle], properties.style)}
        />
    );
}
