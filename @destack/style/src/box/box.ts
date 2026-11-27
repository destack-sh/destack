import { schema } from "@destack/schema";
import { Dimension } from "../dimension/index.ts";
import { type Bindable } from "../reference/index.ts";

/** The radius of each corner of a box, as CSS rounds them. */
const cornersJson = schema
    .object({
        /** The top left corner's radius. */
        topLeft: Dimension.bindable,
        /** The top right corner's radius. */
        topRight: Dimension.bindable,
        /** The bottom right corner's radius. */
        bottomRight: Dimension.bindable,
        /** The bottom left corner's radius. */
        bottomLeft: Dimension.bindable,
    })
    .meta({ id: "style.corners", title: "Corners" });
/** The radius of each corner of a box. */
export type Corners = schema.Infer<typeof cornersJson>;

/** The radius of each corner of a box, as CSS rounds them. */
export const Corners = Object.assign(cornersJson, {
    /** Round every corner by one radius. */
    all(radius: Bindable<Dimension>): Corners {
        return { topLeft: radius, topRight: radius, bottomRight: radius, bottomLeft: radius };
    },

    /** Write the corners as CSS's `border-radius`, clockwise from the top left. */
    css(corners: Corners): string {
        const radii = [corners.topLeft, corners.topRight, corners.bottomRight, corners.bottomLeft];

        return radii.map((radius) => Dimension.css(radius)).join(" ");
    },
});

/** A length at each side of a box, as CSS pads, offsets and insets them. */
const boxSidesJson = schema
    .object({
        /** The top side's length. */
        top: Dimension.bindable,
        /** The right side's length. */
        right: Dimension.bindable,
        /** The bottom side's length. */
        bottom: Dimension.bindable,
        /** The left side's length. */
        left: Dimension.bindable,
    })
    .meta({ id: "style.boxSides", title: "Sides" });
/** A length at each side of a box. */
export type BoxSides = schema.Infer<typeof boxSidesJson>;

/** A length at each side of a box, as CSS pads, offsets and insets them. */
export const BoxSides = Object.assign(boxSidesJson, {
    /** Give every side one length. */
    all(length: Bindable<Dimension>): BoxSides {
        return { top: length, right: length, bottom: length, left: length };
    },

    /** Write the sides as CSS writes `padding` or `margin`, clockwise from the top. */
    css(sides: BoxSides): string {
        const lengths = [sides.top, sides.right, sides.bottom, sides.left];

        return lengths.map((length) => Dimension.css(length)).join(" ");
    },
});
