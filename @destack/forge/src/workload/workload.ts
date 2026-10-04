import { workloadIdentity } from "@destack/account/client";
import { ResourceHandle } from "@destack/resource";
import { defineWorkload } from "@destack/service/workload";
import { ForgeServer, type ForgeServerOptions } from "../server/index.ts";
import { forgeDatabase } from "../stack/index.ts";

/** What the process running the forge binds: its store of builds, the URL clients call it at, its storage of platform repositories and its GitHub App. */
export type ForgeConfiguration = Pick<
    ForgeServerOptions,
    "store" | "endpoint" | "storage" | "github"
>;

/** The forge's configuration, which the process placing it binds. */
export const forgeConfiguration = new ResourceHandle<ForgeConfiguration>(
    import.meta.destack.package,
    "configuration",
);

/** The forge: repositories and their references, packages with their releases and builds, and the npm endpoints, one per residency. */
export const forgeWorkload = defineWorkload({
    name: "forge",
    placement: ["per-residency"],
    start: (context) => {
        // serve the forge over its database, following the account service as its placement
        const forge = new ForgeServer({
            ...forgeConfiguration.get(context.resources),
            database: forgeDatabase.get(context.resources),
            identity: workloadIdentity.get(context.resources),
            callKey: context.callKey,
            report: (error) => context.report(error),
        });

        return { services: [forge.service()] };
    },
});
