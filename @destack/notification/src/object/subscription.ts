import { space } from "@destack/account/object";
import { intersection, relation, through } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { ObjectReference, type Subject } from "@destack/sync";

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
            subscribe: through("parent", "read"),
        },
        constraints: {
            owner: { kind: "unique", on: ["parentPackageId", "parentType", "parentId", "ownerId"] },
        },
    },
    (method) => ({
        /** Subscribe the caller to the parent, once. */
        create: method.create("own", { fields: [] }),
        /** Find a principal's subscription to the parent, or subscribe it. */
        findOrCreate: method.create("subscribe", { visibility: "installation" }),
        list: method.list("own"),
        delete: method.delete("own"),
    }),
);

/** A subscription as its table stores it. */
export type Subscription = Select<typeof subscription.table>;

/** The subscriptions of principals to objects. */
export const Subscription = {
    /** Build the call subscribing a principal to an object for a reason, once. */
    subscribe(
        host: Pick<ObjectReference, "packageId" | "type" | "id">,
        owner: Subject,
        reason: Reason,
    ) {
        return subscription.call("findOrCreate", {
            parent: { packageId: host.packageId, type: host.type, id: host.id },
            ...ObjectReference.columns("owner", owner),
            reason,
        });
    },
};
