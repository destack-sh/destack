import { type Package, type Target } from "@destack/package";
import { type Runtime } from "@destack/package/runtime";
import { schema } from "@destack/schema";
import { type ModuleDescription, ModuleGraph } from "@destack/package/code";
import {
    createPackageInspection,
    DeclarationDescription,
    type PackageInspection,
} from "@destack/package/inspect";
import { TestDeclaration } from "@destack/test/inspect";

/** Source and resolution settings for package inspection. */
export interface InspectOptions {
    /** The source package directory. */
    directory: string;
    /** The selected execution target. */
    target: Target;
    /** Runtime conditions used to resolve dependencies. */
    runtime?: Runtime;
    /** The TypeScript configuration; omit to use package defaults. */
    configuration?: string;
}

/** Collect code and tests, then evaluate exported domain declarations. */
export async function inspectPackage(options: InspectOptions): Promise<PackageInspection> {
    // start an isolated compiler
    const { PackageBuilder } = await import("../build/builder.ts");
    const { directory, ...request } = options;
    await using compiler = await PackageBuilder.start(directory);

    return await compiler.inspect(request);
}

/** Describe the modules and declarations collected by the compiler. */
export function inspectModules(
    source: Package,
    modules: ModuleDescription[],
    tests: TestDeclaration[],
    declarations: DeclarationDescription[],
): PackageInspection {
    return createPackageInspection(
        source.name,
        new ModuleGraph(modules),
        schema.object({
            tests: schema.array(TestDeclaration),
            declarations: schema.array(DeclarationDescription),
        }),
        { tests, declarations },
    );
}
