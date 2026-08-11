import type { HostEvent } from "../_generated/runtime/host/event/event.js";
import type * as hostService from "../_generated/runtime/service/host.js";
import type { Instant } from "../_generated/runtime/world/time/instant.js";
import { HostClient } from "../_generated/world/host.js";

import type { World } from "./world.js";

/** Runtime-controlled Clock values. */
export type ClockValue = Pick<hostService.Clock, "wall" | "monotonic">;

/** One World-bound controllable host. */
export class Host {
    /** Owning World. */
    readonly world: World;
    /** Complete generated Host service client. */
    readonly client: HostClient;

    /** World-bound Clock. */
    readonly clock: Clock;
    /** World-bound Random source. */
    readonly random: Random;

    /** Bind one Host to its World. */
    constructor(world: World) {
        this.world = world;
        this.client = new HostClient(world.connection);

        this.clock = new Clock(this);
        this.random = new Random(this);
    }

    /** Send one typed host Event into this World. */
    async send(event: HostEvent): Promise<void> {
        await this.client.sendEvent({ worldId: this.world.id, event });
    }
}

/** One World-bound Clock. */
export class Clock {
    /** Owning Host. */
    readonly host: Host;

    /** Bind one Clock to its Host. */
    constructor(host: Host) {
        this.host = host;
    }

    /** Read the effective Clock. */
    async read(): Promise<hostService.Clock> {
        const response = await this.host.client.readClock({
            worldId: this.host.world.id,
        });

        return response.value;
    }

    /** Advance the runtime-controlled Clock to one wall-clock deadline. */
    async advance(deadline: Instant): Promise<hostService.Clock> {
        const response = await this.host.client.advanceClock({
            worldId: this.host.world.id,
            deadline,
        });

        return response.value;
    }

    /** Replace runtime-controlled wall and monotonic Clock state. */
    async set(value: ClockValue): Promise<hostService.Clock> {
        const response = await this.host.client.setClock({
            worldId: this.host.world.id,
            ...value,
        });

        return response.value;
    }
}

/** One World-bound Random source. */
export class Random {
    /** Owning Host. */
    readonly host: Host;

    /** Bind one Random source to its Host. */
    constructor(host: Host) {
        this.host = host;
    }

    /** Read the effective Random source. */
    async read(): Promise<hostService.Random> {
        const response = await this.host.client.readRandom({
            worldId: this.host.world.id,
        });

        return response.value;
    }

    /** Reseed the deterministic Random source. */
    async reseed(seed: bigint): Promise<hostService.Random> {
        const response = await this.host.client.reseedRandom({
            worldId: this.host.world.id,
            seed,
        });

        return response.value;
    }
}

export type { HostEvent };
export { HostClient };
export { hostService } from "../_generated/world/host.js";
