import { schema } from "@destack/schema";
import { Color } from "../color/index.ts";
import { Dimension } from "../dimension/index.ts";

/** A shadow a box casts, or one inside it, as the design token format's shadow. */
const shadowJson = schema
    .object({
        /** The shadow's color. */
        color: Color.schema,
        /** How far it falls to the right. */
        offsetX: Dimension,
        /** How far it falls down. */
        offsetY: Dimension,
        /** How far it blurs. */
        blur: Dimension,
        /** How far it grows past the box before it blurs. */
        spread: Dimension,
        /** Whether it falls inside the box, as the format and CSS's `inset` keyword mark it. */
        inset: schema.boolean(),
    })
    .meta({ id: "style.shadow", title: "Shadow" });
/** A shadow a box casts, or one inside it. */
export type Shadow = schema.Infer<typeof shadowJson>;

/** A shadow a box casts, or one inside it, as the design token format's shadow. */
export const Shadow = Object.assign(shadowJson, {
    /** Write a shadow as one of CSS's `box-shadow` list. */
    css(shadow: Shadow): string {
        // write the lengths in CSS's order, inset first where it falls inside
        const lengths = [shadow.offsetX, shadow.offsetY, shadow.blur, shadow.spread];
        const written = lengths.map((length) => Dimension.css(length));

        return [...(shadow.inset ? ["inset"] : []), ...written, shadow.color].join(" ");
    },
});

/** The shadows of a box, the first on top, as CSS lists them. */
export const ShadowStack = Object.assign(
    schema.array(Shadow).meta({ id: "style.shadowStack", title: "Shadows" }),
    {
        /** Write the shadows as CSS's `box-shadow`, none for no shadow. */
        css(stack: readonly Shadow[]): string {
            const shadows = stack.map((shadow) => Shadow.css(shadow));

            return shadows.length === 0 ? "none" : shadows.join(", ");
        },
    },
);
