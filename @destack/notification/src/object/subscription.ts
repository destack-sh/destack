import { space } from "@destack/account/object";
import { Subject } from "@destack/sync";
import { intersection, relation, through } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field, ParentReference } from "@destack/object";
import { schema } from "@destack/schema";

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
            // the source's readers subscribe others and read the subscribers their calls notify
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
        subscribers: method.query({
            permission: "add",
            target: "scope",
            isInternal: true,
            input: schema.object({
                /** The object subscribed to. */
                parent: ParentReference,
                /** The last subscription read before, absent for the first page. */
                after: schema.identifier("subscription").exactOptional(),
                /** The most subscribers read. */
                limit: schema.int().min(1),
                /** The latest subscription time kept. */
                until: schema.number().int().nonnegative().exactOptional(),
            }),
            output: schema.object({
                /** The subscribers, oldest first. */
                items: schema.array(
                    schema.object({
                        /** The subscription. */
                        id: schema.identifier("subscription"),
                        /** The subscriber. */
                        owner: Subject,
                        /** Why the principal is subscribed. */
                        reason: schema.enum(REASONS),
                    }),
                ),
            }),
        }),
        delete: method.delete("own"),
    }),
);

/** A subscription as its table stores it. */
export type Subscription = Select<typeof subscription.table>;
