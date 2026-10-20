export { defineProcedure, type ProcedureAccess } from "../procedure/procedure.ts";

export * from "../declare/index.ts";

/** Describe event streams. */
export { eventIterator } from "@orpc/contract";
export { getEventMeta, withEventMeta } from "@orpc/client";

export type {
    AnyContractRouter as ProcedureRouter,
    ContractRouterClient as Client,
    Route,
} from "@orpc/contract";
