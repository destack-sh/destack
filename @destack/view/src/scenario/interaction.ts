import type { Interaction } from "@destack/package/declare";
import { defineSchema, schema } from "@destack/schema";
import { Observation } from "./observation.ts";
import { Step } from "./step.ts";

/** The theme settings, locale, direction and width a host renders UI examples in. */
export const Environment = defineSchema(
    schema.object({
        /** The theme settings by name, such as `appearance: "dark"`, each setting's default when absent. */
        theme: schema.record(schema.string().min(1), schema.json()).exactOptional(),
        /** The language and region, such as `ar-EG`, the source language when absent. */
        locale: schema.string().min(1).exactOptional(),
        /** The text direction, the locale's own when absent. */
        direction: schema.enum(["ltr", "rtl"]).exactOptional(),
        /** The container width in CSS pixels, the host's own when absent. */
        width: schema.number().int().positive().exactOptional(),
    }),
);
/** The theme settings, locale, direction and width a host renders UI examples in. */
export type Environment = schema.Infer<typeof Environment>;

/** The UI interaction: Playwright's actions, locators and assertions, played in process by `ViewDriver`. */
export const viewInteraction: Interaction<Step, Observation, Environment> = Object.freeze({
    name: "ui",
    step: Step,
    observation: Observation,
    environment: Environment,
});
