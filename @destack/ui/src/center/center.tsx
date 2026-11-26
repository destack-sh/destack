import { SPACE } from "../position/index.ts";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, merge, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The widest a centered column grows and its gutters unless set. */
const DEFAULTS = { max: "65ch" } as const;

/** The styles of a centered column. */
const styles = style.create({
    center: {
        boxSizing: "content-box",
        marginInline: "auto",
    },
    intrinsic: {
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
    },
    text: {
        textAlign: "center",
    },
    layout: (max: string, gutters: string) => ({
        maxInlineSize: max,
        paddingInline: gutters,
    }),
});

/** The properties people change in a center's controls and inspector. */
const CENTER_PROPERTIES = {
    gutters: SPACE.labeled(t.context("label")`Gutters`)
        .described(t`The space on each side at narrow widths.`)
        .default("4"),
    intrinsic: field
        .boolean()
        .labeled(t.context("label")`Intrinsic`)
        .described(t`Centers the elements inside at their own widths too.`)
        .default(false),
    andText: field
        .boolean()
        .labeled(t.context("label")`Center text`)
        .described(t`Centers the text inside too.`)
        .default(false),
};

/** The properties of a center besides its declared ones: its width and attributes. */
export interface CenterAttributes
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style">, ElementPartProperties {
    /** The widest the column grows, any CSS length. */
    readonly max?: string;
}

/** Center a column in its container up to a maximum width, with gutters at narrow widths. */
export const Center = defineComponent({
    name: "center",
    properties: CENTER_PROPERTIES,
    render(properties: RenderProperties<CenterAttributes, typeof CENTER_PROPERTIES>): JSX.Element {
        const center = merge(DEFAULTS, properties);

        return renderPart(
            "div",
            "center",
            omit(center, "max", "gutters", "intrinsic", "andText"),
            () => [
                styles.center,
                center.intrinsic && styles.intrinsic,
                center.andText && styles.text,
                styles.layout(center.max, space[center.gutters]),
            ],
        );
    },
});

/** The properties of a center, its native attributes included. */
export type CenterProperties = ComponentProperties<typeof Center>;
