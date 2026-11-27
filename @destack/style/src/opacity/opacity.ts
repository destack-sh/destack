import { schema } from "@destack/schema";

/** How much a box covers what lies behind it, from 0 to 1, as CSS's opacity. */
export const Opacity = schema
    .number()
    .min(0)
    .max(1)
    .meta({ id: "style.opacity", title: "Opacity" });
/** How much a box covers what lies behind it. */
export type Opacity = schema.Infer<typeof Opacity>;
