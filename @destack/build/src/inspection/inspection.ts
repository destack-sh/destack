import { type PackageVersion, DeclarationDescription } from "@destack/package";
import { type Runtime } from "@destack/package/runtime";
import { schema, type Version } from "@destack/schema";
import { type ModuleDescription, ModuleGraph, PackageInspection } from "@destack/package/code";
import { ExampleDeclaration } from "../graph/example.ts";
import { TestDeclaration } from "../outline/test.ts";

/** Source and resolution settings for package inspection. */
export interface InspectOptions {
    /** The source package directory. */
    directory: string;
    /** The runtime the source is inspected for. */
    runtime: Runtime;
    /** The TypeScript configuration, absent for the package defaults. */
    configuration?: string;
    /** The versions the inspected build releases its source packages at, by package name. */
    versions?: Readonly<Record<string, Version>>;
}

/** The contents of an inspection beside its modules: tests, examples and declarations. */
export const InspectionContents = schema.object({
    /** The tests and suites the modules declare. */
    tests: schema.array(TestDeclaration),
    /** The examples the modules export. */
    examples: schema.array(ExampleDeclaration),
    /** The declarations the export entries evaluate to. */
    declarations: schema.array(DeclarationDescription),
});
/** The contents of an inspection beside its modules: tests, examples and declarations. */
export type InspectionContents = schema.Infer<typeof InspectionContents>;

/** Describe the modules, tests, examples and declarations collected by the compiler. */
export function inspectModules(
    source: PackageVersion,
    modules: ModuleDescription[],
    contents: InspectionContents,
): PackageInspection {
    return PackageInspection.create(
        source.name,
        new ModuleGraph(modules),
        InspectionContents,
        contents,
    );
}
