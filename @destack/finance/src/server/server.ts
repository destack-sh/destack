import { implementCustomer } from "./customer.ts";
import { implementCharge } from "./charge.ts";
import { implementCheckoutSession } from "./checkout-session.ts";
import { implementPrice } from "./price.ts";
import { implementProduct } from "./product.ts";
import { implementProductFeature } from "./product-feature.ts";
import { implementSeller } from "./seller.ts";
import { implementSubscription } from "./subscription.ts";
import { Billing } from "./billing.ts";
import type { Metering } from "./metering.ts";
import { CallError } from "@destack/runtime/error";
import type { ServerDefinition } from "@destack/object/server";
import financePackage from "../package.ts";
import { type PaymentProvider, ProviderEvent } from "../provider/provider.ts";
import { EntitlementController } from "./entitlement-controller.ts";
import { UsageController } from "./usage-controller.ts";

/** Implement the finance package in a home: its types, the provider's webhook, its controllers. */
export function implementFinance(
    home: string,
    metering: Metering,
    charging: PaymentProvider | undefined,
): ServerDefinition<typeof financePackage> {
    const billing = new Billing(charging, home);

    return {
        objects: [
            implementSeller(billing),
            implementProduct(),
            implementProductFeature(metering),
            implementPrice(metering),
            implementSubscription(billing),
            implementCheckoutSession(billing),
            implementCustomer(metering),
            implementCharge(metering),
        ],

        // take Stripe's webhook deliveries, refusing them where finance bills through no provider
        adapters: {
            stripe: async (request, _caller, _context, store) => {
                if (charging === undefined) {
                    throw new CallError("NOT_FOUND", {
                        message: "finance bills through no payment provider",
                    });
                } else if (request.method !== "POST") {
                    return new Response(null, { status: 405 });
                }
                await ProviderEvent.receive(home, charging, request, store);

                return new Response(null, { status: 204 });
            },
        },

        // rate the usage routed here and derive each account's entitlements
        controllers: [
            (store) => new UsageController(store, home),
            (store) => new EntitlementController(store, home, metering),
        ],
    };
}
