import { workloadIdentity } from "@destack/account/client";
import { ResourceHandle } from "@destack/resource";
import { defineWorkload } from "@destack/service/workload";
import { FinanceServer, type FinanceServerOptions } from "../server/index.ts";
import { financeDatabase } from "../stack/index.ts";

/** What the process running the finance service binds: how it opens the builds declaring features and meters. */
export type FinanceConfiguration = Pick<FinanceServerOptions, "release">;

/** The finance service's configuration, which the process placing it binds. */
export const financeConfiguration = new ResourceHandle<FinanceConfiguration>(
    import.meta.destack.package,
    "configuration",
);

/** The finance service: the billing objects and entitlements of a residency's accounts, one per residency. */
export const financeWorkload = defineWorkload({
    name: "finance",
    placement: ["residency"],
    start: (context) => {
        // serve the finance objects over their database, following the account service as its placement
        // TODO #Incomplete: open releases through the forge
        // TODO #Incomplete: charge through Stripe and receive its webhook deliveries as system calls
        const finance = new FinanceServer({
            ...financeConfiguration.get(context.resources),
            database: financeDatabase.get(context.resources),
            identity: workloadIdentity.get(context.resources),
            callKey: context.callKey,
            report: (error) => context.report(error),
        });

        return { services: [finance.service()] };
    },
});
