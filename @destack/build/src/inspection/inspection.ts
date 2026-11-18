import { type PackageVersion, DeclarationDescription } from "@destack/package";
import { type Runtime } from "@destack/package/runtime";
import { schema, type Version } from "@destack/schema";
import { type ModuleDescription, ModuleGraph, PackageInspection } from "@destack/package/code";
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

/** Describe the modules and declarations collected by the compiler. */
export function inspectModules(
    source: PackageVersion,
    modules: ModuleDescription[],
    tests: TestDeclaration[],
    declarations: DeclarationDescription[],
): PackageInspection {
    return PackageInspection.create(
        source.name,
        new ModuleGraph(modules),
        schema.object({
            tests: schema.array(TestDeclaration),
            declarations: schema.array(DeclarationDescription),
        }),
        { tests, declarations },
    );
}
