import { space } from "@destack/account/object";
import { intersection, relation, through } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";

/** Why a principal hears about an object, as code forges' notification reasons. */
export const REASONS = ["author", "mention", "assigned", "participating", "subscribed"] as const;

/** Why a principal hears about an object. */
export type Reason = (typeof REASONS)[number];

/** A principal's subscription to an object's notifications. */
export const subscription = defineObject(
    {
        name: "subscription",
        plural: "subscriptions",
        scope: space,
        nested: { in: "any", receive: "subscribe" },
        fields: {
            /** The subscriber. */
            owner: field.subject().caller(),
            /** Why the principal is subscribed. */
            reason: field.enum(REASONS).default("subscribed" satisfies Reason),
        },
        permissions: {
            own: intersection(relation("owner"), through("parent", "subscribe")),
            // the source's readers subscribe others
            add: through("parent", "read"),
        },
        constraints: {
            owner: { kind: "unique", on: ["parentPackageId", "parentType", "parentId", "ownerId"] },
        },
    },
    (method) => ({
        create: method.create("own", { fields: [] }),
        add: method.create("add", { isInternal: true }),
        list: method.list("own"),
        delete: method.delete("own"),
    }),
);

/** A subscription as its table stores it. */
export type Subscription = Select<typeof subscription.table>;
