export { defineProcedure, type ProcedureAccess } from "../procedure/procedure.ts";

export * from "../declare/index.ts";

/** Describe event streams. */
export { eventIterator } from "@orpc/contract";
export { withEventMeta } from "@orpc/client";

export type {
    AnyContractRouter as ServiceRouter,
    ContractRouterClient as Client,
    Route,
} from "@orpc/contract";
