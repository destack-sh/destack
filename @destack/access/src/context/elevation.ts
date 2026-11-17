import { schema } from "@destack/schema";
import type { AuthenticationAssurance } from "./context.ts";

/** The highest authentication assurance level: phishing-resistant factors. */
export const HIGHEST_ASSURANCE = 3;

/** The schema of an elevation: the assurance level a caller authenticated at, and how recently. */
const elevationSchema = schema.object({
    /** The lowest assurance: 1 for one factor, 2 for several, 3 for phishing-resistant. */
    assurance: schema.number().int().min(1).max(HIGHEST_ASSURANCE),
    /** The longest time since the caller authenticated at that level, in milliseconds. */
    maxAge: schema.number().int().positive(),
});

/** The authentication of a sensitive permission: an assurance level achieved recently enough. */
export type Elevation = schema.Infer<typeof elevationSchema>;

/** The schema of a step-up challenge: assurance level, and elevation age if set (RFC 9470). */
const stepUpSchema = schema.object({
    /** The lowest assurance: 1 for one factor, 2 for several, 3 for phishing-resistant. */
    assurance: schema.number().int().min(1).max(HIGHEST_ASSURANCE),
    /** The longest time since the caller authenticated at that level, in milliseconds. */
    maxAge: schema.number().int().positive().exactOptional(),
});

/** The authentication a step-up asks of a refused caller: assurance level and elevation age. */
export type StepUp = schema.Infer<typeof stepUpSchema>;

/** The authentication a step-up challenge asks of a refused caller. */
export const StepUp = {
    /** The schema of a step-up challenge. */
    schema: stepUpSchema,

    /** Write a step-up as a URL's query, as a request for a stepped-up token sends it. */
    query(stepUp: StepUp): URLSearchParams {
        const { assurance, maxAge } = stepUp;

        return new URLSearchParams({
            assurance: String(assurance),
            ...(maxAge === undefined ? {} : { maxAge: String(maxAge) }),
        });
    },

    /** Read the step-up a URL's query asks, or nothing where it asks none. */
    ofQuery(query: URLSearchParams): StepUp | undefined {
        // read no step-up from a query without an assurance
        const assurance = query.get("assurance");
        const maxAge = query.get("maxAge");
        if (assurance === null) {
            return undefined;
        }

        return stepUpSchema.parse({
            assurance: Number(assurance),
            ...(maxAge === null ? {} : { maxAge: Number(maxAge) }),
        });
    },
};

/** The authentication of a sensitive permission, like OpenID Connect `acr_values` and `max_age`. */
export const Elevation = {
    /** The schema of an elevation. */
    schema: elevationSchema,

    /** Report whether an authentication meets the elevation's level before its age ends. */
    meets(
        elevation: Elevation,
        assurance: AuthenticationAssurance | undefined,
        now: number,
    ): boolean {
        return (
            assurance !== undefined &&
            assurance.level >= elevation.assurance &&
            now - assurance.authenticatedAt < elevation.maxAge
        );
    },

    /** Read the moment an accepted authentication grows too old for the elevation. */
    until(
        elevation: Elevation,
        assurance: AuthenticationAssurance | undefined,
        now: number,
    ): number | undefined {
        return assurance !== undefined && Elevation.meets(elevation, assurance, now)
            ? assurance.authenticatedAt + elevation.maxAge
            : undefined;
    },
};
