import { implementCustomer } from "./customer.ts";
import { FinanceError } from "../error/error.ts";
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
import { Scope, RequestId } from "@destack/sync";
import type { ImplementationInput } from "@destack/object/server";
import type { ServerContext } from "@destack/object/workload";
import { stripeWebhook } from "../declare/webhook.ts";
import financePackage from "../package.ts";
import { type PaymentProvider, ProviderEvent } from "../provider/provider.ts";
import { CustomerEntitlement } from "./entitlement.ts";
import { usage } from "@destack/stack/object";
import { charge } from "../object/index.ts";
import type {} from "@destack/package/import-meta";

/** Implement the finance package in a home: its types, the provider's webhook, its cursors. */
export function implementFinance(
    home: string,
    metering: Metering,
    charging: PaymentProvider | undefined,
    server: Pick<ServerContext, "resources">,
): ImplementationInput<typeof financePackage> {
    // take the provider's verified deliveries, and refuse them without a provider
    const billing = new Billing(charging, home);
    const webhook =
        charging === undefined
            ? undefined
            : stripeWebhook.serve(
                  (delivery, store) => ProviderEvent.receive(home, charging, delivery, store),
                  server,
              );

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

        adapters: {
            stripe:
                webhook ??
                (async () => {
                    throw FinanceError.of("PROVIDER_MISSING", "finance has no payment provider");
                }),
        },

        // follow each account's entitlements from the universe, which the derivations compare against
        follows: [
            entitlementShape.per(account, {
                key: async (row) => ({ key: { scope: Scope.universe.id, account: row.id } }),
            }),
        ],

        // rate the usage routed here and derive each account's entitlements
        cursors: [
            // read each payer's usage, held thirty days past any rating outage
            usage.cursor(
                "usage",
                async (page, { store, transaction }) => {
                    // rate the claimed page through the charges' method, moving the cursor in the same commit
                    await store.mutate(
                        store.as(charge),
                        {
                            id: RequestId.create(store.environment),
                            calls: [
                                charge.call("rate", {
                                    ...charge.in(home),
                                    accountId: account.identifier(page.scope),
                                    after: page.after,
                                    through: page.through,
                                }),
                            ],
                        },
                        { transaction, scope: home },
                    );
                },
                { hold: { days: 30 } },
            ),
            (store) => new CustomerEntitlement(store, home, metering).projection(),
        ],
    };
}
