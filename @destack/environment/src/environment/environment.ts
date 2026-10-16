import { Clock } from "../clock/clock.ts";
import { Network } from "../network/network.ts";

/** The effects a node runs on: its clock and its network. */
export interface Environment {
    /** The time. */
    readonly clock: Clock;
    /** The network. */
    readonly network: Network;
}

/** Environments. */
export const Environment = {
    /** The system's clock and network. */
    system: { clock: Clock.system, network: Network.system } satisfies Environment,
};
