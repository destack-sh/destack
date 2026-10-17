import { schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { subscription } from "../object/subscription.ts";

/** Implement subscriptions: one per principal and object, paged by object. */
export function implementSubscription(): typeof subscription {
    return subscription.implement({
        methods: {
            add: async (call, next) => {
                // answer the principal's subscription to the parent, or subscribe it
                const parent = call.requireParent();
                const [existing] = await call.read(subscription).findMany({
                    where: {
                        parentPackageId: parent.packageId,
                        parentType: parent.type,
                        parentId: parent.id,
                        ownerId: schema.string().parse(call.input["ownerId"]),
                    },
                });

                return existing ?? next();
            },
            subscribers: async (call) => {
                // read a page of the object's subscribers, oldest first
                const { parent, after, limit, until } = call.input;
                const rows = await call.read(subscription).findMany({
                    where: {
                        parentPackageId: parent.packageId,
                        parentType: parent.type,
                        parentId: parent.id,
                        ...(after === undefined ? {} : { id: { gt: after } }),
                        ...(until === undefined ? {} : { createdAt: { lte: until } }),
                    },
                    orderBy: { id: "asc" },
                    limit,
                });
                const items = rows.map((row) => ({
                    id: row.id,
                    owner: ObjectReference.at("owner", row),
                    reason: row.reason,
                }));

                return { items };
            },
        },
    });
}
