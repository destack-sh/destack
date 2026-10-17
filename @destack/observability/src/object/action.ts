import { through, union } from "@destack/access";
import { space } from "@destack/account/object";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { installation } from "@destack/stack/object";
import { AttributeKey, PersonalAttributes } from "../event/attribute.ts";
import { ANALYTICS_RETENTION, TELEMETRY_FLUSH } from "../event/policy.ts";

/** What an action keeps beside its keys. */
export const ActionData = schema.object({
    /** The properties the emitter marked personal. */
    personal: PersonalAttributes.exactOptional(),
});

/** Something a person did in an application, as the application tracks it by a literal name. */
export const action = defineObject(
    {
        name: "action",
        plural: "actions",
        scope: space,
        storage: {
            kind: "events",
            flush: TELEMETRY_FLUSH,
            retention: ANALYTICS_RETENTION,
            delivery: "at-most-once",
            subject: "person",
        },
        fields: {
            /** The action's personal properties, masked unless the reader may unmask them. */
            data: field.json(ActionData).guard({ read: "unmask" }),
            /** The installation that tracked it. */
            installation: field.reference(installation).optional(),
            /** The digest of the tracking build's manifest. */
            build: field.string().optional(),
            /** The action's name. */
            name: field.string(),
            /** The view's route pattern, absent outside a view. */
            route: field.string().optional(),
            /** The session the action belongs to. */
            session: field.string().optional(),
            /** The trace the action was tracked in. */
            trace: field.string().optional(),
            /** The visitor, as a hash. */
            visitor: field.string().optional(),
            /** The signed-in person, whose key seals their personal values. */
            person: field.string().optional(),
            /** The action's properties, the personal ones left out. */
            attributes: field.json(AttributeKey),
        },
        permissions: {
            read: union(through("installation", "analyze"), through("scope", "analyze")),
            unmask: union(through("installation", "unmask"), through("scope", "unmask")),
            // let the machine serving the scope append what it forwards
            append: through("scope", "serve"),
        },
    },
    (method) => ({
        /** Append the records an export carries. */
        append: method.createMany("append", {
            fields: [
                "installationId",
                "build",
                "name",
                "route",
                "session",
                "trace",
                "visitor",
                "person",
                "attributes",
                "data",
            ],
        }),
    }),
);
