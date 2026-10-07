import { workloadIdentity } from "@destack/account/client";
import { ResourceHandle } from "@destack/resource";
import { defineWorkload } from "@destack/service/workload";
import { EventStore } from "@destack/event";
import { implementEvents } from "@destack/event/server";
import { admitUsage, type FinanceOptions, implementFinance } from "../server/index.ts";
import { usage } from "../meter/usage.ts";
import { financeDatabase, usageBucket } from "../stack/index.ts";

/** What the process running the finance service binds. */
export type FinanceConfiguration = Pick<FinanceOptions, "registry" | "tag" | "provider">;

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
        // keep the routed usage in the database and the usage bucket
        const database = financeDatabase.get(context.resources);
        const files = usageBucket.get(context.resources);
        const events = new EventStore({
            database,
            kinds: [usage],
            files: () => files,
            report: (error) => context.report(error),
        });

        // serve the finance objects over their database, following the account service as its placement
        const finance = implementFinance({
            ...financeConfiguration.get(context.resources),
            database,
            events,
            identity: workloadIdentity.get(context.resources),
            callKey: context.callKey,
            report: (error) => context.report(error),
        });

        // receive the usage routed to the residency's accounts from the workloads serving them
        const routed = implementEvents({
            store: events,
            access: finance.objects.access,
            admit: admitUsage(finance.objects),
        });

        return { services: [finance, routed] };
    },
});
