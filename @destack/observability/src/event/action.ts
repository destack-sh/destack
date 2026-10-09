import { defineEventKind } from "@destack/event/declare";
import { ANALYTICS_ACCESS } from "./access.ts";
import { schema } from "@destack/schema";
import { AttributeKey, PersonalAttributes } from "./attribute.ts";
import { TELEMETRY_FLUSH } from "./policy.ts";
import { ANALYTICS_RETENTION } from "./visit.ts";

/** Something a person did in an application, as the application tracks it by a literal name. */
export const action = defineEventKind({
    name: "action",
    description: "Something a person did in an application.",
    keys: schema.object({
        /** The installation that tracked it. */
        installation: schema.string().nullable(),
        /** The digest of the tracking build's manifest. */
        build: schema.string().nullable(),
        /** The action's name. */
        name: schema.string(),
        /** The view's route pattern, null outside a view. */
        route: schema.string().nullable(),
        /** The session the action belongs to. */
        session: schema.string().nullable(),
        /** The trace the action was tracked in. */
        trace: schema.string().nullable(),
        /** The visitor: a hash of the day's salt, the installation, the address and the user agent. */
        visitor: schema.string().nullable(),
        /** The signed-in person, whose key seals their personal values. */
        person: schema.string().nullable(),
        /** The action's properties, the personal ones left out. */
        attributes: AttributeKey,
    }),
    data: schema.object({
        /** The properties the emitter marked personal. */
        personal: PersonalAttributes.exactOptional(),
    }),
    delivery: "at-most-once",
    policy: { flush: TELEMETRY_FLUSH, retention: ANALYTICS_RETENTION },
    access: ANALYTICS_ACCESS,
    subject: "person",
});
