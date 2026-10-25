import { present } from "@destack/schema";
import { Subject } from "@destack/sync";
import { NotificationError } from "../error/error.ts";
import type { MethodContext } from "@destack/object";
import { type Subscription, subscription } from "../object/subscription.ts";

/** Implement subscriptions: one per principal and object, paged by object. */
export function implementSubscription(): typeof subscription {
    return subscription.implement({
        methods: {
            findOrCreate: async (call, next) => {
                // refuse a principal who cannot read the parent
                const parent = call.requireParent();
                const { input } = call;
                const owner = Subject.parse({
                    packageId: input["ownerPackageId"],
                    type: input["ownerType"],
                    scope: input["ownerScope"],
                    id: input["ownerId"],
                });
                const host = present(
                    call.objects.find((object) => object.policy.is(parent)),
                    `the type of ${parent.type} ${parent.id}`,
                );
                if (!(await call.permits(owner, host.permission("read"), parent))) {
                    throw NotificationError.of(
                        "RECIPIENT_FORBIDDEN",
                        `${owner.type} ${owner.id} cannot read ${parent.type} ${parent.id}`,
                    );
                }

                // answer the principal's subscription to the parent, or subscribe it
                return (await find(call, owner)) ?? next();
            },
            create: async (call, next) => {
                // answer the caller's subscription to the parent, or subscribe it
                return (await find(call, call.caller.subject)) ?? next();
            },
        },
    });
}

/** Find a principal's subscription to the call's parent, absent before one. */
async function find(call: MethodContext, owner: Subject): Promise<Subscription | undefined> {
    const parent = call.requireParent();
    const [existing] = await call.read(subscription).findMany({
        where: {
            parentPackageId: parent.packageId,
            parentType: parent.type,
            parentId: parent.id,
            ownerId: owner.id,
        },
    });

    return existing;
}
