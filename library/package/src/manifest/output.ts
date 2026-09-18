import { defineSchema, schema } from "@destack/schema";
import { DependencyName, DependencyResolution, Target } from "../package/index.ts";
import { PackagePath } from "../file/index.ts";
import { PackageInspectionReference } from "./inspection.ts";

/** Compiled files and dependencies for one execution target. */
export const PackageOutput = defineSchema(schema.object({
    /** The execution target. */
    target: Target,
    /** The output directory within the build. */
    directory: PackagePath,
    /** Generated entrypoints keyed by their exported names. */
    exports: schema.record(schema.string().min(1), PackagePath),
    /** Exact dependencies retained as runtime imports. */
    dependencies: schema.record(DependencyName, DependencyResolution),
    /** Inspection documents collected for this target. */
    inspections: schema.array(PackageInspectionReference),
}));

/** Compiled files for one execution target. */
export type PackageOutput = schema.Infer<typeof PackageOutput>;
