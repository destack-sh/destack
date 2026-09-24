import { defineSchema, schema } from "@destack/schema";
import { DependencyName, DependencyRelease, Target } from "../definition/index.ts";
import { PackagePath } from "../file/index.ts";
import { Runtime } from "../runtime/index.ts";
import { WorkloadDescription } from "../workload/index.ts";
import { DeclarationName } from "../definition/package.ts";

/** Compiled files and dependencies for one execution target. */
export const PackageOutput = defineSchema(
    schema.object({
        /** The execution target. */
        target: Target,
        /** The concrete compiler runtime selected for this output. */
        runtime: Runtime,
        /** Whether this output is distributed for execution. */
        emit: schema.boolean(),
        /** The declared view compiled into this output. */
        view: DeclarationName.optional(),
        /** Validated workloads keyed by their declared names. */
        workloads: schema.record(DeclarationName, WorkloadDescription),
        /** Domain collection names and selected record indices. */
        descriptions: schema.record(schema.string(), schema.array(schema.number().int().min(0))),
        /** The output directory within the build. */
        directory: PackagePath,
        /** Generated entrypoints keyed by their exported names. */
        exports: schema.record(schema.string().min(1), PackagePath),
        /** Exact dependencies referenced by runtime code. */
        dependencies: schema.record(DependencyName, DependencyRelease),
    }),
);

/** Compiled files for one execution target. */
export type PackageOutput = schema.Infer<typeof PackageOutput>;
