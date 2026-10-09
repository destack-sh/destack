import { defineEventKind } from "@destack/event/declare";
import { ANALYTICS_ACCESS } from "./access.ts";
import { schema } from "@destack/schema";
import { PersonalAttributes } from "./attribute.ts";
import { DAY, TELEMETRY_FLUSH } from "./policy.ts";

/** How long analytics stay unless a space's setting says otherwise: 13 months, as the CNIL guideline keeps audience measurement. */
export const ANALYTICS_RETENTION = 395 * DAY;

/** A view a person opened: anonymous by a daily-rotating salted hash, signed in by their identity as personal data. */
export const visit = defineEventKind({
    name: "visit",
    description: "A view a person opened.",
    keys: schema.object({
        /** The installation whose view was opened. */
        installation: schema.string().nullable(),
        /** The digest of the view's build manifest. */
        build: schema.string().nullable(),
        /** The view's route pattern. */
        route: schema.string(),
        /** The session the visit belongs to. */
        session: schema.string().nullable(),
        /** The trace the visit was recorded in. */
        trace: schema.string().nullable(),
        /** The visitor: a hash of the day's salt, the installation, the address and the user agent. */
        visitor: schema.string().nullable(),
        /** The signed-in person, whose key seals their personal values. */
        person: schema.string().nullable(),
    }),
    data: schema.object({
        /** The path opened. */
        path: schema.string().exactOptional(),
        /** The host the visitor came from. */
        referrer: schema.string().exactOptional(),
        /** The page's title. */
        title: schema.string().exactOptional(),
        /** The visitor's locale. */
        locale: schema.string().exactOptional(),
        /** The attributes the emitter marked personal. */
        personal: PersonalAttributes.exactOptional(),
    }),
    delivery: "at-most-once",
    policy: { flush: TELEMETRY_FLUSH, retention: ANALYTICS_RETENTION },
    access: ANALYTICS_ACCESS,
    subject: "person",
});
