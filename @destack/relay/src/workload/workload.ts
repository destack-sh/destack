import { ResourceHandle } from "@destack/resource";
import { defineWorkload } from "@destack/service/workload";
import { implementRelay, RELAY_PACKAGE, type RelayOptions } from "../server/index.ts";
import { accounts, relayDatabase } from "../stack/index.ts";
import { present } from "@destack/schema";

/** What the machine running a relay provides it. */
export type RelayConfiguration = Pick<
    RelayOptions,
    "origin" | "tokens" | "domains" | "fetch" | "destinations"
>;

/** The relay's configuration, which the process placing it binds. */
export const relayConfiguration = new ResourceHandle<RelayConfiguration>(
    RELAY_PACKAGE,
    "configuration",
);

/** The edge of Destack's names: a relay installation per residency. */
export const relayWorkload = defineWorkload({
    name: "relay",
    start: (context) => {
        // route names at the root and keep each partition's tunnels
        const relay = implementRelay({
            ...relayConfiguration.get(context.resources),
            database: relayDatabase.get(context.resources),
            accounts: accounts.get(context.resources),
            installation: present(context.installation, "the relay installation"),
            hibernation: present(context.hibernation, "the hibernation of the relay's host"),
            callKey: context.callKey,
            clock: context.clock,
            ...(context.history === undefined ? {} : { history: context.history }),
            report: (error) => context.report(error),
        });

        return { services: [relay], sockets: relay.tunnels };
    },
});
