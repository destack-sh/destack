import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, merge, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The space, alignment and justification of a cluster unless set. */
const DEFAULTS = { space: "3", justify: "start", align: "center" } as const;

/** The flexbox value of each justification. */
const JUSTIFY = {
    start: "flex-start",
    center: "center",
    end: "flex-end",
    between: "space-between",
} as const;

/** The flexbox value of each alignment. */
const ALIGN = {
    start: "flex-start",
    center: "center",
    end: "flex-end",
    baseline: "baseline",
    stretch: "stretch",
} as const;

/** The styles of a cluster. */
const styles = style.create({
    cluster: {
        display: "flex",
        flexWrap: "wrap",
    },
    layout: (gap: string, justify: string, align: string) => ({
        gap,
        justifyContent: justify,
        alignItems: align,
    }),
});

/** The properties of a cluster, the native element's attributes included. */
export interface ClusterProperties
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style">, ElementPartProperties {
    /** The space between the elements and between their rows, a spacing step. */
    readonly space?: keyof typeof space;
    /** Where each row's elements sit along it. */
    readonly justify?: keyof typeof JUSTIFY;
    /** Where elements of different heights sit within their row. */
    readonly align?: keyof typeof ALIGN;
}

/** Lay elements out in a row that wraps onto more rows, with one space between each. */
export function Cluster(properties: ClusterProperties): JSX.Element {
    const cluster = merge(DEFAULTS, properties);

    return renderPart("div", "cluster", omit(cluster, "space", "justify", "align"), () => [
        styles.cluster,
        styles.layout(space[cluster.space], JUSTIFY[cluster.justify], ALIGN[cluster.align]),
    ]);
}
