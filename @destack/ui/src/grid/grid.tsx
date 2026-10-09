import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, merge, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The narrowest column and the space between cells unless set, a column of about 30 characters. */
const DEFAULTS = { min: "16rem", space: "4" } as const;

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

/** The properties of a grid, the native element's attributes included. */
export interface GridProperties
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style">, ElementPartProperties {
    /** The narrowest a column gets before the grid drops one, any CSS length. */
    readonly min?: string;
    /** The space between cells, a spacing step. */
    readonly space?: keyof typeof space;
}

/** Lay elements out in as many equal columns as fit, each at least the minimum wide. */
export function Grid(properties: GridProperties): JSX.Element {
    const grid = merge(DEFAULTS, properties);

    return renderPart("div", "grid", omit(grid, "min", "space"), () => [
        styles.grid,
        styles.layout(grid.min, space[grid.space]),
    ]);
}
