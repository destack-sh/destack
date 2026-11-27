import { schema } from "@destack/schema";
import { Dimension } from "../dimension/index.ts";

/** The radius of each corner of a box, as CSS rounds them. */
const cornersJson = schema
    .object({
        /** The top left corner's radius. */
        topLeft: Dimension,
        /** The top right corner's radius. */
        topRight: Dimension,
        /** The bottom right corner's radius. */
        bottomRight: Dimension,
        /** The bottom left corner's radius. */
        bottomLeft: Dimension,
    })
    .meta({ id: "corners", title: "Corners" });
/** The radius of each corner of a box. */
export type Corners = schema.Infer<typeof cornersJson>;

/** The radius of each corner of a box, as CSS rounds them. */
export const Corners = Object.assign(cornersJson, {
    /** Round every corner by one radius. */
    all(radius: string): Corners {
        return { topLeft: radius, topRight: radius, bottomRight: radius, bottomLeft: radius };
    },

    /** Write the corners as CSS's `border-radius`, clockwise from the top left. */
    css(corners: Corners): string {
        return [corners.topLeft, corners.topRight, corners.bottomRight, corners.bottomLeft].join(
            " ",
        );
    },
});

/** A length at each side of a box, as CSS pads, offsets and insets them. */
const boxSidesJson = schema
    .object({
        /** The top side's length. */
        top: Dimension,
        /** The right side's length. */
        right: Dimension,
        /** The bottom side's length. */
        bottom: Dimension,
        /** The left side's length. */
        left: Dimension,
    })
    .meta({ id: "boxSides", title: "Sides" });
/** A length at each side of a box. */
export type BoxSides = schema.Infer<typeof boxSidesJson>;

/** A length at each side of a box, as CSS pads, offsets and insets them. */
export const BoxSides = Object.assign(boxSidesJson, {
    /** Give every side one length. */
    all(length: string): BoxSides {
        return { top: length, right: length, bottom: length, left: length };
    },

    /** Write the sides as CSS writes `padding` or `margin`, clockwise from the top. */
    css(sides: BoxSides): string {
        return [sides.top, sides.right, sides.bottom, sides.left].join(" ");
    },
});
