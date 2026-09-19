/** Define procedures, HTTP routes, schemas, errors, and shared metadata. */
export { oc as procedure } from "@orpc/contract";

export * from "./declaration.ts";

/** Describe validated server event streams. */
export { eventIterator } from "@orpc/contract";

export type {
    AnyContractRouter as Service,
    ContractRouterClient as Client,
    InferContractRouterInputs as ServiceInputs,
    InferContractRouterOutputs as ServiceOutputs,
    Route,
} from "@orpc/contract";
