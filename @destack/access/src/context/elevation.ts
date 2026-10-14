import { defineSchema, schema } from "@destack/schema";
import type { AccessContext } from "./context.ts";

/** The highest authentication assurance level: phishing-resistant factors. */
export const HIGHEST_ASSURANCE = 3;

/** The schema of an elevation: the assurance level a caller authenticated at, and how recently. */
const elevationSchema = defineSchema(
    schema.object({
        /** The lowest assurance: 1 for one factor, 2 for several, 3 for phishing-resistant. */
        assurance: schema.number().int().min(1).max(HIGHEST_ASSURANCE),
        /** The longest time since the caller authenticated at that level, in milliseconds. */
        maxAge: schema.number().int().positive(),
    }),
);

/** The authentication of a sensitive permission: an assurance level reached recently enough. */
export type Elevation = schema.Infer<typeof elevationSchema>;

/** The schema of a step-up challenge: assurance level, and elevation age if set (RFC 9470). */
const stepUpSchema = defineSchema(
    schema.object({
        /** The lowest assurance: 1 for one factor, 2 for several, 3 for phishing-resistant. */
        assurance: schema.number().int().min(1).max(HIGHEST_ASSURANCE),
        /** The longest time since the caller authenticated at that level, in milliseconds. */
        maxAge: schema.number().int().positive().exactOptional(),
    }),
);

/** The authentication a step-up asks of a refused caller: assurance level and elevation age. */
export type StepUp = schema.Infer<typeof stepUpSchema>;

/** The authentication a step-up challenge asks of a refused caller. */
export const StepUp = {
    /** The schema of a step-up challenge. */
    schema: stepUpSchema,
};

/** The authentication of a sensitive permission, like OpenID Connect `acr_values` and `max_age`. */
export const Elevation = {
    /** The schema of an elevation. */
    schema: elevationSchema,

    /** Report whether a request authenticated at the elevation's level within its age. */
    admits(elevation: Elevation, context: AccessContext): boolean {
        const assurance = context.assurance;

        return (
            assurance !== undefined &&
            assurance.level >= elevation.assurance &&
            context.now - assurance.authenticatedAt <= elevation.maxAge
        );
    },

    /** Read the moment an admitted request's authentication grows too old for the elevation. */
    until(elevation: Elevation, context: AccessContext): number | undefined {
        const assurance = context.assurance;

        return assurance !== undefined && Elevation.admits(elevation, context)
            ? assurance.authenticatedAt + elevation.maxAge
            : undefined;
    },
};
