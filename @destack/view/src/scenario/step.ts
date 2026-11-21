import { schema } from "@destack/schema";
import { Locator } from "./locator.ts";

/** A point relative to an element's top left corner, in CSS pixels. */
const Position = schema.object({
    /** The distance from the left edge. */
    x: schema.number(),
    /** The distance from the top edge. */
    y: schema.number(),
});

/** A button of the pointer. */
const Button = schema.enum(["primary", "secondary"]);

/** One step a UI scenario takes, as data a driver interprets. */
export const Step = schema.discriminatedUnion("action", [
    schema.object({
        /** Move the focus to the element. */
        action: schema.literal("focus"),
        /** The element. */
        target: Locator,
    }),
    schema.object({
        /** Click the element. */
        action: schema.literal("click"),
        /** The element. */
        target: Locator,
        /** The button, the primary one when absent. */
        button: Button.exactOptional(),
        /** The point on the element, its center by default. */
        position: Position.exactOptional(),
    }),
    schema.object({
        /** Move the pointer onto the element. */
        action: schema.literal("pointerMove"),
        /** The element. */
        target: Locator,
        /** The point on the element, its center by default. */
        position: Position.exactOptional(),
    }),
    schema.object({
        /** Press a button of the pointer. */
        action: schema.literal("pointerDown"),
        /** The button, the primary one when absent. */
        button: Button.exactOptional(),
    }),
    schema.object({
        /** Release the button of the pointer. */
        action: schema.literal("pointerUp"),
    }),
    schema.object({
        /** Turn the wheel where the pointer is. */
        action: schema.literal("wheel"),
        /** The horizontal distance, in CSS pixels. */
        deltaX: schema.number().exactOptional(),
        /** The vertical distance, in CSS pixels. */
        deltaY: schema.number().exactOptional(),
    }),
    schema.object({
        /** Press a key, such as `ArrowDown`, `Escape` or `Control+r`. */
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
    schema.object({
        /** Let time pass. */
        action: schema.literal("pause"),
        /** The time, in milliseconds. */
        milliseconds: schema.number().int().positive(),
    }),
]);
/** One step a UI scenario takes, as data a driver interprets. */
export type Step = schema.Infer<typeof Step>;
