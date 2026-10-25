import type { Interaction } from "@destack/package/declare";
import { schema } from "@destack/schema";
import { Observation } from "./observation.ts";
import { Step } from "./step.ts";

/** The theme settings, locale, direction and width a host renders UI examples with. */
export const Globals = schema.object({
    /** The theme settings by name, such as `appearance: "dark"`. */
    theme: schema.record(schema.string().min(1), schema.json()).exactOptional(),
    /** The language and region, such as `ar-EG`, the source language when absent. */
    locale: schema.string().min(1).exactOptional(),
    /** The text direction, the locale's own when absent. */
    direction: schema.enum(["ltr", "rtl"]).exactOptional(),
    /** The container width in CSS pixels, the host's own when absent. */
    width: schema.number().int().positive().exactOptional(),
});
/** The theme settings, locale, direction and width a host renders UI examples with. */
export type Globals = schema.Infer<typeof Globals>;

/** The UI interaction: actions, locators and assertions, played in process by `ViewDriver`. */
export const viewInteraction: Interaction<Step, Observation, Globals> = Object.freeze({
    name: "ui",
    step: Step,
    observation: Observation,
    globals: Globals,
});
