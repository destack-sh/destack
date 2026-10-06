import { defineSchema, schema } from "@destack/schema";
import { Locator } from "./locator.ts";

/** The ARIA states an observation reads, after Playwright's role options. */
export const OBSERVED_STATES = ["checked", "disabled", "expanded", "pressed", "selected"] as const;

/** What a UI driver reads after a step, after Playwright's assertions, as a JSON value. */
export const Observation = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** The accessible name of the focused element, none when nothing has the focus. */
            kind: schema.literal("focused"),
        }),
        schema.object({
            /** The accessible name of the element. */
            kind: schema.literal("name"),
            /** The element. */
            target: Locator,
        }),
        schema.object({
            /** The text of the element. */
            kind: schema.literal("text"),
            /** The element. */
            target: Locator,
        }),
        schema.object({
            /** The text of each matching element, in document order. */
            kind: schema.literal("texts"),
            /** The elements. */
            target: Locator,
        }),
        schema.object({
            /** The value of the field. */
            kind: schema.literal("value"),
            /** The field. */
            target: Locator,
        }),
        schema.object({
            /** An attribute of the element, none when it lacks it. */
            kind: schema.literal("attribute"),
            /** The element. */
            target: Locator,
            /** The attribute's name. */
            name: schema.string().min(1),
        }),
        schema.object({
            /** Whether the element is in an ARIA state. */
            kind: schema.literal("state"),
            /** The element. */
            target: Locator,
            /** The state. */
            state: schema.enum(OBSERVED_STATES),
        }),
        schema.object({
            /** Whether an element matches and shows. */
            kind: schema.literal("visible"),
            /** The element. */
            target: Locator,
        }),
        schema.object({
            /** The number of matching elements. */
            kind: schema.literal("count"),
            /** The elements. */
            target: Locator,
        }),
    ]),
);
/** What a UI driver reads after a step. */
export type Observation = schema.Infer<typeof Observation>;
