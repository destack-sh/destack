import type { Package } from "@destack/package";
import type { DeclarationDescription, BuildDescription } from "@destack/package/inspect";
import type { InspectorName } from "./inspector.ts";

/** An exported declaration located by the compiler. */
export interface Declaration {
    /** The declaration constructor verified by its compiler symbol. */
    inspector: InspectorName;
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
        if (!input.package) {
            continue;
        }
        const files = paths.get(input.package) ?? new Set<string>();
        files.add(input.path.split("?")[0]);
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
