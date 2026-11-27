import { schema } from "@destack/schema";
import { Dimension } from "../dimension/index.ts";

/** How a box is moved, turned and scaled, as CSS's translate, rotate and scale properties do. */
const transformJson = schema
    .object({
        /** How far it moves to the right. */
        x: Dimension.bindable,
        /** How far it moves down. */
        y: Dimension.bindable,
        /** How far it turns clockwise, in degrees. */
        rotate: schema.number().min(-360).max(360),
        /** How much wider it grows, 1 for its width. */
        scaleX: schema.number(),
        /** How much taller it grows, 1 for its height. */
        scaleY: schema.number(),
    })
    .meta({ id: "style.transform", title: "Transform" });
/** How a box is moved, turned and scaled. */
export type Transform = schema.Infer<typeof transformJson>;

/** How a box is moved, turned and scaled, as CSS's translate, rotate and scale properties do. */
export const Transform = Object.assign(transformJson, {
    /** No transform: in place, upright and at its own size. */
    initial: {
        x: { value: 0, unit: "px" },
        y: { value: 0, unit: "px" },
        rotate: 0,
        scaleX: 1,
        scaleY: 1,
    } satisfies Transform,

    /** Move, turn and scale a box as CSS's individual transform properties. */
    css(transform: Transform): TransformStyle {
        return {
            translate: `${Dimension.css(transform.x)} ${Dimension.css(transform.y)}`,
            rotate: `${String(transform.rotate)}deg`,
            scale: `${String(transform.scaleX)} ${String(transform.scaleY)}`,
        };
    },
});

/** A transform as CSS properties. */
export interface TransformStyle {
    /** The move. */
    readonly translate: string;
    /** The turn. */
    readonly rotate: string;
    /** The scale. */
    readonly scale: string;
}
