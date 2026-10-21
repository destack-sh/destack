import { schema } from "@destack/schema";
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
        },
    });
}
