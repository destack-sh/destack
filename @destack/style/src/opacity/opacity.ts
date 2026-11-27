import { schema } from "@destack/schema";

/** How much a box covers what lies behind it, as JSON. */
const opacityJson = schema.number().min(0).max(1).meta({ id: "style.opacity", title: "Opacity" });

/** How much a box covers what lies behind it, from 0 to 1, as CSS's opacity. */
export const Opacity = Object.assign(opacityJson, {
    /** Fully opaque, as CSS starts. */
    initial: 1 satisfies number,
});
/** How much a box covers what lies behind it. */
export type Opacity = schema.Infer<typeof Opacity>;
