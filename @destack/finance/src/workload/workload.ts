import { defineWorkload } from "@destack/service/workload";
import { EventStore } from "@destack/event";
import { implementEvents } from "@destack/event/server";
import { Registry } from "@destack/forge/client";
import { forgeService } from "@destack/forge/service";
import type { ResourceContext } from "@destack/resource/context";
import { present } from "@destack/schema";
import { ServiceMount } from "@destack/service";
import { admitUsage, implementFinance } from "../server/index.ts";
import { usage } from "../meter/usage.ts";
import type { PaymentProvider } from "../provider/index.ts";
import { StripeProvider, stripeSecretKey } from "../stripe/index.ts";
import {
    accounts,
    financeDatabase,
    type FinanceProvider,
    provider,
    tag,
    usageBucket,
} from "../stack/index.ts";

/** The finance service: billing objects and entitlements of a residency's accounts. */
export const financeWorkload = defineWorkload({
    name: "finance",
    start: (context) => {
        // keep the routed usage in the database and the usage bucket
        const installation = present(context.installation, "the finance installation");
        const database = financeDatabase.get(context.resources);
        const files = usageBucket.get(context.resources);
        const events = new EventStore({
            database,
            clock: context.clock,
            kinds: [usage],
            files: () => files,
            report: (error) => context.report(error),
        });

        // read releases through the registry the machine mounts at the forge's package name
        const forge = installation.at(forgeService.package.name);
        const registry = new Registry(async () => ({
            url: `${forge.url}${ServiceMount.path(forgeService.package.id)}`,
            fetch: (request) => forge.fetch(request),
        }));

        // serve the finance objects, copying the residency's accounts from the account service
        const charging = openProvider(
            provider.read(installation.settings),
            context.resources,
            installation.scope,
        );
        const finance = implementFinance({
            database,
            events,
            accounts: accounts.get(context.resources),
            installation,
            callKey: context.callKey,
            clock: context.clock,
            registry,
            tag: tag.read(installation.settings),
            provider: charging,
            report: (error) => context.report(error),
        });

        // receive the usage routed to the residency's accounts from the cloud machines serving them
        const routed = implementEvents({
            store: events,
            access: finance.objects.access,
            admit: admitUsage(finance.objects),
        });

        return { services: [finance, routed] };
    },
});

/** Open the installation's payment provider with its secret for its home, if it has one. */
function openProvider(
    setting: FinanceProvider,
    resources: ResourceContext,
    home: string,
): PaymentProvider | undefined {
    // open no provider
    if (setting === "none") {
        return undefined;
    }

    return new StripeProvider({
        key: stripeSecretKey.get(resources),
        home,
        fetch: (request) => fetch(request),
    });
}
