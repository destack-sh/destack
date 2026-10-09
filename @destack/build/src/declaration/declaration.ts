import { type BuildDescription } from "@destack/package/manifest";
import { modulePath } from "../compile/dependency.ts";
import { type Package, type PackageExport, type DeclarationDescription } from "@destack/package";

/** A function a package exports, located for evaluation. */
export interface FunctionExport {
    /** The package directory. */
    readonly directory: string;
    /** The package export of the function, such as `./inspect`. */
    readonly subpath: string;
    /** The export's package.json target, which may select a module by runtime condition. */
    readonly target: PackageExport;
    /** The function's export name. */
    readonly name: string;
}

/** The functions inspecting one kind of declaration, exported by the kind's packages. */
export interface Inspector {
    /** The function describing a declaration, exported by its constructor's package. */
    readonly describe: FunctionExport;
    /** The function comparing two releases' descriptions, exported by the kind's package. */
    readonly compare?: FunctionExport;
    /** The function listing a description's terms, exported by the kind's package. */
    readonly vocabulary?: FunctionExport;
    /** The function listing the symbols a description derives, exported by the kind's package. */
    readonly symbols?: FunctionExport;
}

/** An exported declaration located by the compiler. */
export interface DeclarationExport {
    /** The functions inspecting the declaration's kind. */
    inspector: Inspector;
    /** The absolute source module path. */
    file: string;
    /** The exported constant name. */
    export: string;
    /** The symbol and source location. */
    description: Omit<DeclarationDescription, "description">;
}

/** Select package declarations and dependency declarations parsed by this compilation. */
export function selectDeclarations(
    declarations: readonly DeclarationDescription[],
    source: Package,
    build: BuildDescription,
): DeclarationDescription[] {
    // index dependency source files, including declarations removed during tree shaking
    const paths = new Map<string, Set<string>>();
    for (const input of Object.values(build.inputs)) {
        if (input.package === undefined) {
            continue;
        }
        const files = paths.get(input.package) ?? new Set<string>();
        files.add(modulePath(input.path));
        paths.set(input.package, files);
    }

    // retain authored declarations and only the dependencies used by this output
    return declarations.filter((declaration) => {
        const owner = declaration.symbol.package;
        if (owner.id === source.id && owner.version === source.version) {
            return true;
        }

        return paths.get(`${owner.name}@${owner.version}`)?.has(declaration.source.file) === true;
    });
}
