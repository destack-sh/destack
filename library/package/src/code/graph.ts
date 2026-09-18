import { PackageError } from "../error/index.ts";
import type { ModuleDescription } from "./module.ts";
import type { SymbolDescription } from "./symbol.ts";
import type { SymbolLocation, SymbolReference } from "./reference.ts";
import type { SourceRange } from "../source/location.ts";
import type { TypeDescription } from "./type.ts";

/** Index modules and resolve symbols within one package inspection. */
export class ModuleGraph {
    /** Modules indexed by package-relative path. */
    readonly modules = new Map<string, ModuleDescription>();
    /** Symbols indexed by module path and qualified name. */
    readonly symbols = new Map<string, Map<string, SymbolDescription>>();

    /** Index modules and check source ranges and symbol references. */
    constructor(modules: readonly ModuleDescription[]) {
        // index symbols before resolving cross-module references
        for (const module of modules) {
            if (this.modules.has(module.path)) {
                throw new PackageError("INVALID_INSPECTION", `Duplicate module: ${module.path}`);
            }
            this.modules.set(module.path, module);
            const symbols = new Map<string, SymbolDescription>();
            for (const symbol of module.symbols) {
                if (symbols.has(symbol.name)) {
                    throw new PackageError(
                        "INVALID_INSPECTION",
                        `Duplicate symbol: ${module.path}#${symbol.name}`,
                    );
                }
                symbols.set(symbol.name, symbol);
            }
            this.symbols.set(module.path, symbols);
        }

        // validate each module against the shared symbol index
        for (const module of this.modules.values()) {
            if (module.source.path !== module.path) {
                throw new PackageError(
                    "INVALID_INSPECTION",
                    `Source file mismatch: ${module.path}`,
                );
            }
            const exports = new Set<string>();
            for (const exported of module.exports) {
                if (exports.has(exported.name)) {
                    throw new PackageError(
                        "INVALID_INSPECTION",
                        `Duplicate export: ${module.path}#${exported.name}`,
                    );
                }
                exports.add(exported.name);
                if ("module" in exported.symbol) this.resolve(exported.symbol);
            }

            // check symbol and member locations against their source modules
            for (const symbol of module.symbols) {
                for (const declaration of symbol.declarations) {
                    this.validateRange(declaration.source);
                }
                for (const member of symbol.members) {
                    for (const declaration of member.declarations) {
                        this.validateRange(declaration.source);
                    }
                }
                for (const reference of references(symbol)) {
                    if ("module" in reference) this.resolve(reference);
                }
            }
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

    /** Check a range against the UTF-16 length of its source module. */
    private validateRange(source: SourceRange): void {
        const module = this.modules.get(source.file);
        if (!module || source.end < source.start || source.end > module.length) {
            throw new PackageError(
                "INVALID_INSPECTION",
                `Invalid source range: ${source.file}:${source.start}:${source.end}`,
            );
        }
    }
}

/** Enumerate named references in a symbol's types and signatures. */
function* references(symbol: SymbolDescription): Generator<SymbolReference> {
    // collect direct types and signatures before visiting their parameters
    const types: TypeDescription[] = [];
    if (symbol.declaredType) types.push(symbol.declaredType);
    if (symbol.valueType) types.push(symbol.valueType);
    const declarations = [
        ...symbol.declarations,
        ...symbol.members.flatMap((member) => member.declarations),
    ];
    const parameters = declarations.flatMap((declaration) => declaration.typeParameters);
    const signatures = [...symbol.signatures, ...symbol.typeSignatures];
    for (const member of symbol.members) {
        types.push(member.type);
        signatures.push(...member.signatures);
    }
    for (const signature of signatures) {
        types.push(signature.returns, ...signature.parameters.map((parameter) => parameter.type));
        if (signature.receiver) types.push(signature.receiver);
        parameters.push(...signature.typeParameters);
    }

    // visit generic bounds, indexes, inherited types, and namespace exports
    for (const parameter of parameters) {
        if (parameter.constraint) types.push(parameter.constraint);
        if (parameter.default) types.push(parameter.default);
    }
    for (const index of symbol.indexes) types.push(index.key, index.value);
    for (const declaration of declarations) {
        for (const base of declaration.heritage) types.push(base.type);
    }
    for (const type of types) yield* type.references;
    for (const exported of symbol.exports) yield exported.symbol;
}
