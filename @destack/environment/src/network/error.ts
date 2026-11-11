import { defineErrorFamily, type ReasonOf } from "@destack/error";

/** The package owning the network's failures, named here as configurations load it unbuilt. */
const OWNER = { name: "@destack/environment" };

/** The network's failures, each reason with the call error code it answers. */
export const NetworkError = defineErrorFamily(
    OWNER,
    {
        // denied
        ADDRESS_PRIVATE_DENIED: {},

        // unsupported
        OPERATION_UNSUPPORTED: {},

        // unavailable
        RESOLVE_UNAVAILABLE: {},
    },
    { audience: "host" },
);

/** A stable cause of network failures. */
export type NetworkReason = ReasonOf<typeof NetworkError>;
