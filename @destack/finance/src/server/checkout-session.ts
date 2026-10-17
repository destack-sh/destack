import { checkoutSession, Customer } from "../object/index.ts";
import { Billing } from "./billing.ts";

/** Implement checkout sessions, which the provider's webhook settles. */
export function implementCheckoutSession(billing: Billing): typeof checkoutSession {
    return checkoutSession.implement({
        methods: {
            create: {
                authorize: (context) => Customer.require(context.input.accountId, context),
                prepare: (context) => billing.prepareCheckout(context),
                handle: (context, next) => billing.openCheckout(context, next),
                settle: (context, settled) => billing.settleCheckout(context, settled),
            },
            settle: (context) => Billing.settle(context),
        },
    });
}
