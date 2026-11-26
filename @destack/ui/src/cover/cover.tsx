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

/** A cover's least height unless set, the viewport's, and its space between and around elements. */
const DEFAULTS = { minHeight: "100svh" } as const;

/** The styles of a cover and its centered element. */
const styles = style.create({
    cover: {
        display: "flex",
        flexDirection: "column",
    },
    layout: (minHeight: string, gap: string, padding: string) => ({
        minBlockSize: minHeight,
        gap,
        padding,
    }),
    center: {
        marginBlock: "auto",
    },
});

/** The properties people change in a cover's controls and inspector. */
const COVER_PROPERTIES = {
    space: SPACE.described(t`The space between the elements and inside the cover's edges.`).default(
        "4",
    ),
    noPad: field
        .boolean()
        .labeled(t.context("label")`No padding`)
        .described(t`Leaves out the space inside the cover's edges.`)
        .default(false),
};

/** The properties of a cover besides its declared ones: its height and attributes. */
export interface CoverAttributes
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style">, ElementPartProperties {
    /** The least height of the cover, any CSS length. */
    readonly minHeight?: string;
}

/** Fill at least a height, centering one element vertically with others above and below it. */
export const Cover = defineComponent({
    name: "cover",
    properties: COVER_PROPERTIES,
    render(properties: RenderProperties<CoverAttributes, typeof COVER_PROPERTIES>): JSX.Element {
        const cover = merge(DEFAULTS, properties);

        return renderPart("div", "cover", omit(cover, "minHeight", "space", "noPad"), () => [
            styles.cover,
            styles.layout(
                cover.minHeight,
                space[cover.space],
                cover.noPad ? "0" : space[cover.space],
            ),
        ]);
    },
});

/** The properties of a cover, its native attributes included. */
export type CoverProperties = ComponentProperties<typeof Cover>;

/** The properties of a cover's centered element, the native element's attributes included. */
export type CoverCenterProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** Render the element a cover centers vertically, such as a page's heading. */
export function CoverCenter(properties: CoverCenterProperties): JSX.Element {
    return renderPart("div", "cover-center", properties, styles.center);
}
