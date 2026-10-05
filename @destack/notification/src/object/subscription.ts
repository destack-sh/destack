import { type ObjectReference, Subject } from "@destack/sync";
import { intersection, relation, through } from "@destack/access";
import { and, eq, gt, lte, unique, type Select } from "@destack/db";
import { type Call, defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { space } from "@destack/space/object";

/** Why a principal hears about an object, as GitHub's notification reasons. */
export const REASONS = ["author", "mention", "assigned", "participating", "subscribed"] as const;

/** Why a principal hears about an object. */
export type Reason = (typeof REASONS)[number];

/** A principal's subscription to an object's notifications. */
export const subscription = defineObject({
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
    constraints: (entry) => [
        unique("subscription_owner").on(
            entry.parentPackageId,
            entry.parentType,
            entry.parentId,
            entry.owner,
        ),
    ],
    permissions: { own: intersection(relation("owner"), through("parent", "subscribe")) },
    methods: (method) => ({
        list: method.list("own"),
        create: method.create("own", { fields: [] }),
        add: method.create(null, { isSystem: true }),
        delete: method.delete("own"),
    }),
});

/** A subscription as its table stores it. */
export type Subscription = Select<typeof subscription.table>;

/** Subscriptions inside a call. */
export const Subscription = {
    /** Report whether a host's type takes subscriptions. */
    isSubscribable(call: Call, host: ObjectReference): boolean {
        const type = call.objects.find((object) => object.policy.is(host));

        return (
            type?.attachments.some((attachment) => subscription.same(attachment.object)) ?? false
        );
    },

    /** Subscribe a principal to a host and keep an existing subscription. */
    async add(call: Call, host: ObjectReference, owner: Subject, reason: Reason): Promise<void> {
        // require a host taking subscriptions
        if (!Subscription.isSubscribable(call, host)) {
            throw new TypeError(`object ${host.type} takes no subscriptions`);
        }

        // subscribe to the host, unless the principal already is
        const table = subscription.table;
        const [existing] = await call.database
            .select({ id: table.id })
            .from(table)
            .where(
                and(
                    eq(table.parentPackageId, host.packageId),
                    eq(table.parentType, host.type),
                    eq(table.parentId, host.id),
                    eq(table.owner, Subject.key(owner)),
                ),
            );
        if (existing === undefined) {
            await call.invoke(subscription).add({
                parent: { packageId: host.packageId, type: host.type, id: host.id },
                owner: Subject.key(owner),
                reason,
            });
        }
    },

    /** Read a page of a host's subscribers, oldest first. */
    async list(
        call: Pick<Call, "database">,
        host: ObjectReference,
        page: {
            readonly after?: string;
            readonly limit: number;
            /** The latest subscription time kept. */
            readonly until?: number;
        },
    ): Promise<
        readonly { readonly id: string; readonly owner: Subject; readonly reason: Reason }[]
    > {
        const table = subscription.table;
        const rows = await call.database
            .select({ id: table.id, owner: table.owner, reason: table.reason })
            .from(table)
            .where(
                and(
                    eq(table.parentPackageId, host.packageId),
                    eq(table.parentType, host.type),
                    eq(table.parentId, host.id),
                    page.after === undefined
                        ? undefined
                        : gt(table.id, schema.identifier("subscription").parse(page.after)),
                    page.until === undefined ? undefined : lte(table.createdAt, page.until),
                ),
            )
            .orderBy(table.id)
            .limit(page.limit);

        return rows.map((row) => ({
            id: row.id,
            owner: Subject.read(row.owner),
            reason: row.reason,
        }));
    },
};
