import { through, union } from "@destack/access";
import { space } from "@destack/account/object";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { installation } from "@destack/stack/object";
import { SESSION_STATUSES } from "@destack/telemetry/otlp";
import { PersonalAttributes } from "../event/attribute.ts";
import { EVENT_FLUSH } from "../event/policy.ts";
import { telemetryRetention } from "../setting/index.ts";

/** What a session keeps beside its keys. */
export const SessionData = schema.object({
    /** The instance that ran it. */
    instance: schema.string().exactOptional(),
    /** The attributes the emitter marked personal. */
    personal: PersonalAttributes.exactOptional(),
});

/** One run of an application a person used, such as a page load, recorded as it ended. */
export const session = defineObject(
    {
        name: "session",
        plural: "sessions",
        scope: space,
        storage: {
            kind: "events",
            flush: EVENT_FLUSH,
            retention: { setting: telemetryRetention.definition },
            delivery: "at-most-once",
            subject: "person",
        },
        fields: {
            /** The session, its personal attributes masked unless the reader may unmask them. */
            data: field.json(SessionData).guard({ read: "unmask" }),
            /** The installation that ran it. */
            installation: field.reference(installation).optional(),
            /** The digest of the build that ran it, which release health groups by. */
            build: field.string().optional(),
            /** How it ended: without failures, with handled ones, or crashed by an escaped one. */
            status: field.enum(SESSION_STATUSES),
            /** The visitor, as a hash. */
            visitor: field.string().optional(),
            /** The signed-in person, whose key seals their personal values. */
            person: field.string().optional(),
        },
        permissions: {
            read: union(through("installation", "analyze"), through("scope", "analyze")),
            unmask: union(through("installation", "unmask"), through("scope", "unmask")),
            // let the machine serving the scope append what it forwards
            append: through("scope", "serve"),
        },
    },
    (method) => ({
        /** Append the sessions an export ends. */
        append: method.createMany("append", {
            fields: ["installationId", "build", "status", "visitor", "person", "data"],
        }),
    }),
);
