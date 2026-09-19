import { defineSchema, schema } from "@destack/schema";
import { DependencyName, DependencyRelease, Target } from "../package/index.ts";
import { PackagePath } from "../file/index.ts";
import { PackageInspectionReference } from "./inspection.ts";
import { Runtime } from "../runtime/index.ts";
import { WorkloadDescription } from "../workload/index.ts";
import { DeclarationDescription } from "../inspect/declaration.ts";
import { ResourceName } from "@destack/resource";

/** Compiled files and dependencies for one execution target. */
export const PackageOutput = defineSchema(schema.object({
    /** The execution target. */
    target: Target,
    /** The concrete compiler runtime selected for this output. */
    runtime: Runtime,
    /** Validated workloads keyed by their declared names. */
    workloads: schema.record(ResourceName, WorkloadDescription),
    /** Collected resource, secret, service and schedule declarations. */
    declarations: schema.array(DeclarationDescription),
    /** The output directory within the build. */
    directory: PackagePath,
    /** Generated entrypoints keyed by their exported names. */
    exports: schema.record(schema.string().min(1), PackagePath),
    /** Exact dependencies retained as runtime imports. */
    dependencies: schema.record(DependencyName, DependencyRelease),
    /** Inspection documents collected for this target. */
    inspections: schema.array(PackageInspectionReference),
}));

/** Compiled files for one execution target. */
export type PackageOutput = schema.Infer<typeof PackageOutput>;
