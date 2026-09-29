import { defineSchema, schema } from "@destack/schema";
import type { AccessContext } from "./context.ts";

/** The highest authentication assurance level: phishing-resistant factors. */
export const HIGHEST_ASSURANCE = 3;

/** The schema of an elevation: the assurance level a caller authenticated at, and how recently. */
const elevationSchema = defineSchema(
    schema.object({
        /** The lowest assurance level: 1 for one factor, 2 for several, 3 for phishing-resistant factors. */
        assurance: schema.number().int().min(1).max(HIGHEST_ASSURANCE),
        /** The longest time since the caller authenticated at that level, in milliseconds. */
        maxAge: schema.number().int().positive(),
    }),
);
/** The authentication a sensitive permission requires: an assurance level reached recently enough. */
export type Elevation = schema.Infer<typeof elevationSchema>;

/** The authentication a step-up challenge asks of a refused caller: the assurance level and elevation age. */
export type StepUp = Pick<Elevation, "assurance"> & Partial<Pick<Elevation, "maxAge">>;

/** The authentication a sensitive permission requires, like OpenID Connect's `acr_values` and `max_age`. */
export const Elevation = {
    /** The schema of an elevation. */
    schema: elevationSchema,
    admits,
    until,
};

/** Report whether a request authenticated at the elevation's level within its age. */
function admits(elevation: Elevation, context: AccessContext): boolean {
    const assurance = context.assurance;

    return (
        assurance !== undefined &&
        assurance.level >= elevation.assurance &&
        context.now - assurance.authenticatedAt <= elevation.maxAge
    );
}

/** Read the moment an admitted request's authentication grows too old for the elevation. */
function until(elevation: Elevation, context: AccessContext): number | undefined {
    return admits(elevation, context)
        ? context.assurance!.authenticatedAt + elevation.maxAge
        : undefined;
}
