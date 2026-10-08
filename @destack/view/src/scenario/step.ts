import { defineSchema, schema } from "@destack/schema";
import { Locator } from "./locator.ts";

/** One step a UI scenario takes, as data a driver interprets. */
export const Step = defineSchema(
    schema.discriminatedUnion("action", [
        schema.object({
            /** Move the focus to the element. */
            action: schema.literal("focus"),
            /** The element. */
            target: Locator,
        }),
        schema.object({
            /** Press and release the main pointer button on the element, focusing it as a pointer does. */
            action: schema.literal("click"),
            /** The element. */
            target: Locator,
        }),
        schema.object({
            /** Press the secondary pointer button on the element, opening its context menu. */
            action: schema.literal("rightClick"),
            /** The element. */
            target: Locator,
            /** The point relative to the element's top left corner, its center when absent. */
            position: schema
                .object({
                    /** The distance from the left edge in CSS pixels. */
                    x: schema.number(),
                    /** The distance from the top edge in CSS pixels. */
                    y: schema.number(),
                })
                .exactOptional(),
        }),
        schema.object({
            /** Press a key, such as `ArrowDown`, `Escape` or `Control+r`, its modifiers joined by plus signs. */
            action: schema.literal("press"),
            /** The key with its modifiers. */
            key: schema.string().min(1),
            /** The element to focus first, the focused element when absent. */
            target: Locator.exactOptional(),
        }),
        schema.object({
            /** Replace a field's value as a person typing it would. */
            action: schema.literal("fill"),
            /** The field. */
            target: Locator,
            /** The new value. */
            value: schema.string(),
        }),
    ]),
);
/** One step a UI scenario takes, as data a driver interprets. */
export type Step = schema.Infer<typeof Step>;
