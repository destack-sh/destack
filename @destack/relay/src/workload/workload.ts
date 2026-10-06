import type { RoleRequest } from "@destack/access";
import { workloadIdentity } from "@destack/account/client";
import { account, key, machine, zone } from "@destack/account/object";
import { ResourceHandle } from "@destack/resource";
import { defineWorkload } from "@destack/service/workload";
import { implementRelay, RELAY_PACKAGE, type Relay, type RelayOptions } from "../server/index.ts";
import { relayDatabase } from "../stack/index.ts";

/** What the process running a relay binds: its origin, the verifier of machines' tokens, its domains and reach to regions, and the runtime serving it. */
export interface RelayConfiguration extends Pick<
    RelayOptions,
    "origin" | "tokens" | "domains" | "fetch"
> {
    /** Serve the started relay on the runtime's WebSockets, such as a Bun listener or a Durable Object, until closed. */
    serve(relay: Relay): { close(): Promise<void> };
}

/** The universe role operators bind the relay's workload: reading the accounts, machines, machine keys and zones names resolve with. */
export const RELAY_ROLE = {
    name: "relay",
    description: "Route Destack's names to the machines and regions serving them",
    permissions: [
        account.permission("read"),
        machine.permission("read"),
        key.permission("read"),
        zone.permission("read"),
    ],
} as const satisfies RoleRequest;

/** The relay's configuration, which the process placing it binds. */
export const relayConfiguration = new ResourceHandle<RelayConfiguration>(
    RELAY_PACKAGE,
    "configuration",
);

/** The edge of Destack's names: one relay per universe, following the account service as its placement. */
export const relayWorkload = defineWorkload({
    name: "relay",
    placement: ["universe"],
    start: (context) => {
        // follow the account service as its placement, serving names and tunnels on the runtime's WebSockets
        const configuration = relayConfiguration.get(context.resources);
        const relay = implementRelay({
            origin: configuration.origin,
            tokens: configuration.tokens,
            ...(configuration.domains === undefined ? {} : { domains: configuration.domains }),
            ...(configuration.fetch === undefined ? {} : { fetch: configuration.fetch }),
            database: relayDatabase.get(context.resources),
            identity: workloadIdentity.get(context.resources),
            report: (error) => context.report(error),
        });
        const serving = configuration.serve(relay.relay);
        context.defer(() => serving.close());

        return { services: [relay] };
    },
});
