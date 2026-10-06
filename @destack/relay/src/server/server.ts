import type { ObjectServer } from "@destack/object/server";
import type { ServiceImplementation } from "@destack/service/server";
import { relayService } from "../service/index.ts";
import { NameController, Relay, type RelayOptions } from "./relay.ts";

/** The relay's service with its copies and the relay routing names and keeping tunnels. */
export interface RelayImplementation extends ServiceImplementation {
    /** The copies of the rows names resolve with. */
    readonly objects: ObjectServer;
    /** The relay routing names and keeping its machines' tunnels. */
    readonly relay: Relay;
}

/** Implement the relay's service: follow the copies names resolve with, telling connected machines their changed names. */
export function implementRelay(options: RelayOptions): RelayImplementation {
    // follow the copies, serving no procedures since machines reach the relay through its runtime's WebSockets
    const relay = new Relay(options);

    return {
        ...relay.objects.implement(relayService, [new NameController(relay)]),
        router: {},
        objects: relay.objects,
        relay,
    };
}
