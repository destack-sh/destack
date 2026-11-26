import { SPACE } from "../position/index.ts";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, merge, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The narrowest column and the space between cells unless set, a column of about 30 characters. */
const DEFAULTS = { min: "16rem" } as const;

/** The styles of a grid. */
const styles = style.create({
    grid: {
        display: "grid",
    },
    layout: (min: string, gap: string) => ({
        gridTemplateColumns: `repeat(auto-fit, minmax(min(${min}, 100%), 1fr))`,
        gap,
    }),
});

/** The properties people change in a grid's controls and inspector. */
const GRID_PROPERTIES = {
    space: SPACE.described(t`The space between cells.`).default("4"),
};

/** The properties of a grid besides its declared ones: its cell width and attributes. */
export interface GridAttributes
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style">, ElementPartProperties {
    /** The narrowest a column gets before the grid drops one, any CSS length. */
    readonly min?: string;
}

/** Lay elements out in as many equal columns as fit, each at least the minimum wide. */
export const Grid = defineComponent({
    name: "grid",
    properties: GRID_PROPERTIES,
    render(properties: RenderProperties<GridAttributes, typeof GRID_PROPERTIES>): JSX.Element {
        const grid = merge(DEFAULTS, properties);

        return renderPart("div", "grid", omit(grid, "min", "space"), () => [
            styles.grid,
            styles.layout(grid.min, space[grid.space]),
        ]);
    },
});

/** The properties of a grid, its native attributes included. */
export type GridProperties = ComponentProperties<typeof Grid>;
