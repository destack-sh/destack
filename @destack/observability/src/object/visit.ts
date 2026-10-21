import { through, union } from "@destack/access";
import { space } from "@destack/account/object";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { installation } from "@destack/stack/object";
import { PersonalAttributes } from "../event/attribute.ts";
import { EVENT_FLUSH } from "../event/policy.ts";
import { SettingValue } from "@destack/setting/object";
import { analyticsRetention } from "../setting/index.ts";

/** What a visit keeps beside its keys. */
export const VisitData = schema.object({
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
});

/** A view a person opened. */
export const visit = defineObject(
    {
        name: "visit",
        plural: "visits",
        scope: space,
        storage: {
            kind: "events",
            flush: EVENT_FLUSH,
            retention: SettingValue.retention(analyticsRetention),
            delivery: "at-most-once",
            subject: "person",
        },
        fields: {
            /** The visit, its personal attributes masked unless the reader may unmask them. */
            data: field.json(VisitData).guard({ read: "unmask" }),
            /** The installation whose view was opened. */
            installation: field.reference(installation).optional(),
            /** The digest of the view's build manifest. */
            build: field.string().optional(),
            /** The view's route pattern. */
            route: field.string(),
            /** The session the visit belongs to. */
            session: field.string().optional(),
            /** The trace the visit was recorded in. */
            trace: field.string().optional(),
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
        /** Append the records an export carries. */
        append: method.createMany("append", {
            fields: [
                "installationId",
                "build",
                "route",
                "session",
                "trace",
                "visitor",
                "person",
                "data",
            ],
        }),
    }),
);
