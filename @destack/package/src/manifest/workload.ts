import { schema } from "@destack/schema";
import { DeclarationName, Entrypoint } from "../definition/package.ts";
import { DeclarationReference } from "../declare/declaration.ts";
import { ComputeDefinition } from "../definition/compute.ts";
import { Sandbox, SandboxName } from "../definition/sandbox.ts";

/** A unit of deployment, as its declaration defines it. */
export const WorkloadDefinition = schema
    .object({
        /** The package-local workload name. */
        name: DeclarationName,
        /** Capacity and lifecycle policy for each instance. */
        compute: ComputeDefinition.exactOptional(),
        /** The package's sandbox abilities the workload uses, every one when omitted. */
        sandbox: schema.array(SandboxName).exactOptional(),
    })
    .strict();
/** A unit of deployment, as its declaration defines it. */
export type WorkloadDefinition = schema.Infer<typeof WorkloadDefinition>;

/** The requests a workload answers: its package's calls, its store's hooks, or its host's views. */
export const WorkloadKind = schema.enum(["call", "hook", "view"]);
/** The requests a workload answers. */
export type WorkloadKind = schema.Infer<typeof WorkloadKind>;

/** A workload located in a compiled output with the declarations its code imports. */
export const WorkloadDescription = schema.object({
    /** The requests the workload answers. */
    kind: WorkloadKind,
    /** The package entrypoint running the workload. */
    entrypoint: Entrypoint,
    /** The handles of this package the workload serves. */
    packages: schema.array(DeclarationReference),
    /** Resource declarations the workload imports. */
    resources: schema.array(DeclarationReference),
    /** Secret declarations the workload imports. */
    secrets: schema.array(DeclarationReference),
    /** Capacity and lifecycle policy for each instance. */
    compute: ComputeDefinition,
    /** What the workload may use beyond the bare sandbox. */
    sandbox: Sandbox,
});
/** A workload located in a compiled output with the declarations its code imports. */
export type WorkloadDescription = schema.Infer<typeof WorkloadDescription>;
