export { defineProcedure, type ProcedureAccess, VERSION_HEADER } from "./procedure.ts";

/** Describe event streams. */
export { eventIterator } from "@orpc/contract";
export { getEventMeta, withEventMeta } from "@orpc/client";

export type {
    AnyContractRouter as ProcedureRouter,
    ContractRouterClient as Client,
    Route,
} from "@orpc/contract";
export * from "./bookmark.ts";
