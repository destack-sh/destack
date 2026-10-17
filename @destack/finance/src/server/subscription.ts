import { Customer, subscription, Subscription } from "../object/index.ts";
import { Billing } from "./billing.ts";

/** Implement subscriptions: they follow the provider, change products and close periods. */
export function implementSubscription(billing: Billing): typeof subscription {
    return subscription.implement({
        methods: {
            create: async (call, next) => {
                await Customer.require(call.input.accountId, call);

                return next();
            },
            closePeriod: {
                prepare: (context) => billing.prepareClose(context),
                handle: (context) => Billing.close(context),
            },
            change: {
                prepare: (context) => billing.prepareChange(context),
                handle: (context) => billing.change(context),
                settle: (context, settled) => billing.settleChange(context, settled),
            },
            sync: (call) => Subscription.sync(call),
        },
    });
}
