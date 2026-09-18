import { defineSchema, schema } from "@destack/schema";
import { PackagePath } from "../file/file.ts";
import { DependencyName, Package } from "../package/package.ts";

/** A symbol name qualified by its source module within a build. */
export const SymbolLocation = defineSchema(schema.object({
    /** The defining source module. */
    module: PackagePath,
    /** The symbol's qualified name within the module. */
    name: schema.string().min(1),
}));
/** A symbol reference within a build. */
export type SymbolLocation = schema.Infer<typeof SymbolLocation>;

/** A symbol in a resolved dependency. */
export const DependencySymbol = defineSchema(schema.object({
    /** The dependency containing the symbol. */
    package: Package.extend({ name: DependencyName }),
    /** The original module and symbol name. */
    symbol: SymbolLocation,
}));

/** A symbol supplied by the language compiler's standard libraries. */
export const CompilerSymbol = defineSchema(schema.object({
    /** The compiler and version that supply the symbol. */
    compiler: Package.extend({ name: DependencyName }),
    /** The library file and qualified symbol name. */
    symbol: SymbolLocation,
}));

/** A local or dependency symbol. */
export const SymbolReference = defineSchema(schema.union([
    SymbolLocation,
    DependencySymbol,
    CompilerSymbol,
]));
/** A local or dependency symbol. */
export type SymbolReference = schema.Infer<typeof SymbolReference>;
