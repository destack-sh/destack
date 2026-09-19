import { PackageError } from "../error/index.ts";
import type { ModuleDescription } from "./module.ts";
import type { SymbolDescription } from "./symbol.ts";
import type { SymbolLocation, SymbolReference } from "./reference.ts";

/** Index modules and resolve symbols within one package inspection. */
export class ModuleGraph {
    /** Modules indexed by package-relative path. */
    readonly modules = new Map<string, ModuleDescription>();
    /** Symbols indexed by module path and qualified name. */
    readonly symbols = new Map<string, Map<string, SymbolDescription>>();

    /** Index modules and their symbols. */
    constructor(modules: readonly ModuleDescription[]) {
        for (const module of modules) {
            this.modules.set(module.path, module);
            this.symbols.set(
                module.path,
                new Map(module.symbols.map((symbol) => [symbol.name, symbol])),
            );
        }
    }

    /** Resolve a symbol or reject an unknown reference. */
    resolve(reference: SymbolLocation): SymbolDescription {
        const symbol = this.symbols.get(reference.module)?.get(reference.name);
        if (!symbol) {
            throw new PackageError(
                "INVALID_INSPECTION",
                `Unresolved symbol: ${reference.module}#${reference.name}`,
            );
        }

        return symbol;
    }

    /** Resolve a public export to its local or dependency symbol. */
    resolveExport(module: string, name: string): SymbolReference {
        const exported = this.modules.get(module)?.exports.find((entry) => entry.name === name);
        if (!exported) {
            throw new PackageError("INVALID_INSPECTION", `Unknown export: ${module}#${name}`);
        }

        return exported.symbol;
    }
}
