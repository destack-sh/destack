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
import { account, entitlementShape } from "@destack/account/object";
import { CallError } from "@destack/object";
import { Scope, RequestId } from "@destack/sync";
import type { ImplementationInput } from "@destack/object/server";
import financePackage from "../package.ts";
import { type PaymentProvider, ProviderEvent } from "../provider/provider.ts";
import { EntitlementController } from "./entitlement-controller.ts";
import { charge, usage } from "../object/index.ts";

/** Implement the finance package in a home: its types, the provider's webhook, its controllers. */
export function implementFinance(
    home: string,
    metering: Metering,
    charging: PaymentProvider | undefined,
): ImplementationInput<typeof financePackage> {
    const billing = new Billing(charging, home);

    return {
        objects: [
            implementSeller(billing),
            implementProduct(),
            implementProductFeature(metering),
            implementPrice(metering),
            implementSubscription(billing),
            implementCheckoutSession(billing),
            implementCustomer(),
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

        // follow each account's entitlements from the universe, which the derivations compare against
        follows: [
            account.follow(entitlementShape, {
                key: async (row) => ({ key: { scope: Scope.universe.id, account: row.id } }),
            }),
        ],

        // rate the usage routed here and derive each account's entitlements
        controllers: [
            usage.consume("usage", async (page, { store, transaction, now }) => {
                // rate the claimed page through the charges' method, moving the reader in the same commit
                await store.mutate(
                    store.serving(charge),
                    {
                        id: RequestId.create(now),
                        calls: [
                            charge.call("rate", {
                                ...charge.in(home),
                                accountId: account.identifier(page.key),
                                after: page.after,
                                through: page.through,
                            }),
                        ],
                    },
                    { transaction, scope: home },
                );
            }),
            (store) => new EntitlementController(store, home, metering),
        ],
    };
}
