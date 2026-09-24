export { defineProcedure, type ProcedureAccess } from "../procedure/procedure.ts";

export * from "../declare/index.ts";

/** Describe validated server event streams. */
export { eventIterator } from "@orpc/contract";

export type {
    AnyContractRouter as ServiceRouter,
    ContractRouterClient as Client,
    InferContractRouterInputs as ServiceInputs,
    InferContractRouterOutputs as ServiceOutputs,
    Route,
} from "@orpc/contract";
