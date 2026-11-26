import { SPACE } from "../position/index.ts";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, merge, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The default width below which a switcher stacks, the space between, and its row limit. */
const DEFAULTS = { threshold: "30rem" } as const;

/** The styles of a switcher. */
const styles = style.create({
    switcher: {
        display: "flex",
        flexWrap: "wrap",
    },
    layout: (threshold: string, gap: string) => ({
        "--switcher-threshold": threshold,
        gap,
    }),
});

/** The properties people change in a switcher's controls and inspector. */
const SWITCHER_PROPERTIES = {
    space: SPACE.described(t`The space between the elements.`).default("4"),
    limit: field
        .integer(schema.number().int().min(1).max(9))
        .labeled(t.context("label")`Limit`)
        .described(t`The most elements in one row, above which they always stack.`)
        .default(4),
};

/** The properties of a switcher besides its declared ones: its threshold and attributes. */
export interface SwitcherAttributes
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style">, ElementPartProperties {
    /** The container width below which the elements stack, any CSS length. */
    readonly threshold?: string;
}

/** Lay elements out in one row of equal widths. */
export const Switcher = defineComponent({
    name: "switcher",
    properties: SWITCHER_PROPERTIES,
    render(
        properties: RenderProperties<SwitcherAttributes, typeof SWITCHER_PROPERTIES>,
    ): JSX.Element {
        const switcher = merge(DEFAULTS, properties);

        return renderPart(
            "div",
            "switcher",
            omit(switcher, "threshold", "space", "limit"),
            () => [styles.switcher, styles.layout(switcher.threshold, space[switcher.space])],
            {
                get "data-limit"() {
                    return switcher.limit;
                },
            },
        );
    },
});

/** The properties of a switcher, its native attributes included. */
export type SwitcherProperties = ComponentProperties<typeof Switcher>;
