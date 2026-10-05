import { defineSchema, schema } from "@destack/schema";
import { DependencyName, DependencyRelease } from "../definition/index.ts";
import { PackagePath } from "../file/index.ts";
import { Runtime } from "../runtime/index.ts";
import { WorkloadDescription } from "./workload.ts";
import { ViewDescription } from "./view.ts";
import { DeclarationName } from "../definition/package.ts";

/** Compiled files and dependencies for one runtime. */
export const PackageOutput = defineSchema(
    schema.object({
        /** The runtime the output is compiled for. */
        runtime: Runtime,
        /** Whether this output is distributed for execution. */
        emit: schema.boolean(),
        /** Validated workloads keyed by their declared names. */
        workloads: schema.record(DeclarationName, WorkloadDescription),
        /** The views this output mounts, keyed by their declared names. */
        views: schema.record(DeclarationName, ViewDescription),
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
