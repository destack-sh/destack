import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { defineExample } from "@destack/package/declare";
import { VirtualList } from "./virtual-list.tsx";

/** The styles of the examples' lists and rows. */
const styles = style.create({
    list: {
        width: "100%",
        blockSize: "20rem",
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
    },
    strip: {
        width: "100%",
        blockSize: "6rem",
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
    },
    row: {
        display: "flex",
        gap: space[3],
        paddingInline: space[3],
        paddingBlock: space[2],
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    muted: {
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
    tile: {
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        blockSize: "100%",
        boxSizing: "border-box",
        borderInlineEndWidth: stroke.border,
        borderInlineEndStyle: "solid",
        borderInlineEndColor: color.border,
    },
    cell: {
        padding: space[2],
        boxSizing: "border-box",
    },
});

/** Ten thousand log lines, every seventh one long enough to wrap. */
const LINES = Array.from({ length: 10_000 }, (_, index) => ({
    id: index + 1,
    message:
        index % 7 === 0
            ? `request ${String(index + 1)} took longer than its budget and was retried on another replica after the first attempt timed out`
            : `request ${String(index + 1)} served`,
}));

/** A log of ten thousand lines of two heights. */
export const virtualListLog = defineExample({
    of: VirtualList,
    name: "log",
    description: t`A log of ten thousand lines of two heights, only the ones in view in the document.`,
    properties: {
        gap: 0,
        "aria-label": "Log",
    },
    render: (properties) => (
        <VirtualList
            each={LINES}
            estimateSize={() => 37}
            gap={properties.gap ?? 0}
            aria-label={properties["aria-label"]}
            xstyle={styles.list}
        >
            {(line) => (
                <div {...style.attrs(text.sm, styles.row)}>
                    <span {...style.attrs(styles.muted)}>{line.id}</span>
                    {line.message}
                </div>
            )}
        </VirtualList>
    ),
});

/** A thousand days side by side. */
export const virtualListDays = defineExample({
    of: VirtualList,
    name: "days",
    description: t`A thousand days side by side, scrolled along the inline axis.`,
    render: () => (
        <VirtualList
            each={LINES.slice(0, 1000)}
            orientation="horizontal"
            estimateSize={() => 96}
            xstyle={styles.strip}
        >
            {(line) => <div {...style.attrs(text.sm, styles.tile)}>Day {line.id}</div>}
        </VirtualList>
    ),
});

/** Ten thousand cells in four columns. */
export const virtualListGrid = defineExample({
    of: VirtualList,
    name: "grid",
    description: t`Ten thousand cells in four lanes with a gap.`,
    render: () => (
        <VirtualList each={LINES} lanes={4} gap={4} estimateSize={() => 40} xstyle={styles.list}>
            {(line) => <div {...style.attrs(text.sm, styles.cell)}>Cell {line.id}</div>}
        </VirtualList>
    ),
});
