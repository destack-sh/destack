import { implementCustomer } from "./customer.ts";
import { FinanceError } from "../error/error.ts";
import { implementCharge } from "./charge.ts";
import { implementCheckoutSession } from "./checkout-session.ts";
import { implementPrice } from "./price.ts";
import { implementProduct } from "./product.ts";
import { implementProductFeature } from "./product-feature.ts";
import { implementSeller } from "./seller.ts";
import { implementSubscription } from "./subscription.ts";
import { accountsShape } from "@destack/account/object";
import { Scope } from "@destack/sync";
import { Billing } from "./billing.ts";
import type { Metering } from "./metering.ts";
import type { ImplementationInput } from "@destack/object/server";
import type { ServerContext } from "@destack/object/workload";
import { stripeWebhook } from "../declare/webhook.ts";
import financePackage from "../package.ts";
import { type PaymentProvider, ProviderEvent } from "../provider/provider.ts";
import type {} from "@destack/package/import-meta";
import { WebhookServer } from "@destack/workflow/server";

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
            : new WebhookServer(
                  stripeWebhook,
                  (delivery, store) => ProviderEvent.receive(home, charging, delivery, store),
                  server,
              ).fetch;

    return {
        objects: [
            implementSeller(billing),
            implementProduct(),
            implementProductFeature(metering),
            implementPrice(metering),
            implementSubscription(billing),
            implementCheckoutSession(billing),
            implementCustomer(home, metering),
            implementCharge(home, metering),
        ],

        mounts: {
            stripe:
                webhook ??
                (async () => {
                    throw FinanceError.of(
                        "PROVIDER_BLOCKED",
                        "cannot bill: finance has no payment provider",
                    );
                }),
        },

        // follow every account and machine from the universe, which billing and quotas authorize by
        // TODO #Architecture: follow a seller's buyers alone once sellers beside the platform sell, standing their rows on each call
        follows: [accountsShape.at({ key: { scope: Scope.universe.id } })],
    };
}
