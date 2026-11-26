import { SPACE } from "../position/index.ts";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

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

/** The properties people change in a cluster's controls and inspector. */
const CLUSTER_PROPERTIES = {
    space: SPACE.described(t`The space between the elements and between their rows.`).default("3"),
    justify: field
        .enum({
            start: { title: t.context("option")`Start` },
            center: { title: t.context("option")`Center` },
            end: { title: t.context("option")`End` },
            between: { title: t.context("option")`Between` },
        })
        .labeled(t.context("label")`Justify`)
        .described(t`Where each row's elements sit along it.`)
        .default("start"),
    align: field
        .enum({
            start: { title: t.context("option")`Start` },
            center: { title: t.context("option")`Center` },
            end: { title: t.context("option")`End` },
            baseline: { title: t.context("option")`Baseline` },
            stretch: { title: t.context("option")`Stretch` },
        })
        .labeled(t.context("label")`Align`)
        .described(t`Where elements of different heights sit within their row.`)
        .default("center"),
};

/** The properties of a cluster besides its declared ones: its native attributes. */
export interface ClusterAttributes
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style">, ElementPartProperties {}

/** Lay elements out in a row that wraps onto more rows, with one space between each. */
export const Cluster = defineComponent({
    name: "cluster",
    properties: CLUSTER_PROPERTIES,
    render(
        properties: RenderProperties<ClusterAttributes, typeof CLUSTER_PROPERTIES>,
    ): JSX.Element {
        return renderPart("div", "cluster", omit(properties, "space", "justify", "align"), () => [
            styles.cluster,
            styles.layout(
                space[properties.space],
                JUSTIFY[properties.justify],
                ALIGN[properties.align],
            ),
        ]);
    },
});

/** The properties of a cluster, its native attributes included. */
export type ClusterProperties = ComponentProperties<typeof Cluster>;
