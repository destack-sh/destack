import type { WorkloadIdentity } from "@destack/account/client";
import { account, key, machine, zone } from "@destack/account/object";
import type { DatabaseConnection } from "@destack/db";
import { type AuditDestination, Journal } from "@destack/audit/server";
import { ObjectServer, Subscriber } from "@destack/object/server";
import type { ServiceImplementation } from "@destack/service/server";
import { relayService } from "../service/index.ts";
import { NameController, Relay, RELAY_PACKAGE, type RelayOptions } from "./relay.ts";

/** What the relay's service runs as: its routing, and the placement following the account service's copies. */
export interface RelayServiceOptions extends Omit<RelayOptions, "directory" | "database"> {
    /** The relay's database, which its copies follow into. */
    readonly database: DatabaseConnection;
    /** The audit history the journal delivers tunnels' openings and closes to, absent where another process delivers them. */
    readonly history?: AuditDestination;
    /** The relay's placement, which follows the account service's copies and reads names' claims and cells' endpoints through its directory. */
    readonly identity: WorkloadIdentity;
}

/** The relay's service with its copies and the relay routing names to its machines' tunnels. */
export interface RelayImplementation extends ServiceImplementation {
    /** The copies of the rows names resolve with. */
    readonly objects: ObjectServer;
    /** The relay routing names to its machines' tunnels. */
    readonly relay: Relay;
}

/** Implement the relay's service: follow the copies names resolve with, telling connected machines their changed names. */
export function implementRelay(options: RelayServiceOptions): RelayImplementation {
    // follow the relay's placement in the account service
    const { database, identity } = options;
    const objects: ObjectServer = new ObjectServer({
        objects: {},
        policies: [account, machine, key, zone],
        database,
        origin: { package: RELAY_PACKAGE, service: relayService.name },
        subscriber: Subscriber.of(identity.publisher(), () =>
            objects.source.workloadSubscriptions(identity.placementId),
        ),
    });

    // route through the universe's directory without procedures
    const relay = new Relay({ ...options, directory: identity.directory() });

    // deliver the journal's tunnel events to the history
    const journal = new Journal(database, options.callKey);

    return {
        ...objects.implement(relayService, [
            new NameController(relay),
            journal.controller(options.history),
        ]),
        router: {},
        objects,
        relay,
    };
}
