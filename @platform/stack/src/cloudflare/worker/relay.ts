import { RELAY_PACKAGE } from "@destack/relay/server";
import { relayDatabase } from "@destack/relay/stack";
import { relayConfiguration, relayWorkload } from "@destack/relay/workload";
import { WorkerdRelay } from "@destack/relay/cloudflare";
import { ResourceContext } from "@destack/resource/context";
import type { Alarm } from "@destack/service/control";
import {
    type DurableInstance,
    type DurableState,
    DurableWorkload,
} from "@destack/service/cloudflare";
import { PlatformAuthentication } from "../../authentication/index.ts";
import { type PlacedEnvironment, PlacedProcess } from "./placed.ts";
import { WorkerProcess } from "./process.ts";

/** The relay process's Durable Object: every name's requests and every host's tunnel, the tunnels open as WebSockets it keeps. */
export class DurableRelay extends DurableWorkload {
    /** Start the relay before the object takes any event. */
    constructor(state: DurableState, environment: PlacedEnvironment) {
        super(state, (alarm) => start(environment, alarm));
    }
}

/** Start the relay as its placement, serving names and tunnels from the object. */
async function start(environment: PlacedEnvironment, alarm: Alarm): Promise<DurableInstance> {
    // verify hosts' relay tokens at the issuer, and keep the relay the workload serves
    const authentication = new PlatformAuthentication(environment.DESTACK_ISSUER, (request) =>
        environment.ACCOUNT.fetch(request),
    );
    const served = Promise.withResolvers<WorkerdRelay>();
    const configuration = new ResourceContext().bind(relayConfiguration, {
        origin: environment.DESTACK_ORIGIN,
        tokens: authentication.verifier(RELAY_PACKAGE.id),
        serve: (relay) => {
            const object = new WorkerdRelay(relay);
            served.resolve(object);

            return object;
        },
    });

    // run the relay workload as its placement, its copies following the account service
    const started = await PlacedProcess.start(
        environment,
        { workload: relayWorkload, database: relayDatabase },
        configuration,
        alarm,
    );
    const relay = await served.promise;

    return {
        fetch: (request) => relay.fetch(request),
        alarm: (deadline) => started.alarm(deadline),
    };
}

/** The relay process's Worker, handing every request to its Durable Object. */
export default {
    /** Forward a request to the process's Durable Object. */
    fetch(request: Request, environment: PlacedEnvironment): Promise<Response> {
        return WorkerProcess.forward(environment, request);
    },
};
