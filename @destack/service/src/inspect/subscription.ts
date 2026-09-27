import { reference } from "@destack/package/declare";
import { SubscriptionDescription, type Subscription } from "../subscription/index.ts";

/** Describe a declared subscription with a reference to its object type. */
export function describeSubscription(subscription: Subscription): SubscriptionDescription {
    return SubscriptionDescription.parse({
        name: subscription.name,
        version: subscription.version,
        object: reference(subscription.object),
        where: subscription.where,
        on: subscription.on,
        from: subscription.from,
        maxLag: subscription.maxLag,
    });
}
