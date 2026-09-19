import { defineSchema, schema } from "@destack/schema";
import { PackagePath } from "../file/file.ts";
import { SymbolReference } from "./reference.ts";
import { SymbolDescription } from "./symbol.ts";
import { PackageFile } from "../file/file.ts";
import { SourceRange } from "../source/location.ts";

/** A runtime global resolved through the language compiler. */
export const GlobalReference = defineSchema(schema.object({
    /** The global identifier used by the module. */
    name: schema.string().min(1),
    /** The type declaration, absent for compiler-intrinsic globals. */
    symbol: SymbolReference.optional(),
    /** Statically selected properties following the global identifier. */
    members: schema.array(schema.string().min(1)),
    /** Whether a computed property requires application execution to resolve. */
    dynamic: schema.boolean(),
    /** The source expression using the global. */
    source: SourceRange,
}));
/** A runtime global resolved through the language compiler. */
export type GlobalReference = schema.Infer<typeof GlobalReference>;

/** A public module export referring to its original symbol. */
export const ExportDescription = defineSchema(schema.object({
    /** The exported name, including default exports. */
    name: schema.string().min(1),
    /** The original symbol. */
    symbol: SymbolReference,
    /** Whether the export exists only in the type system. */
    isTypeOnly: schema.boolean(),
}));

/** A module's symbols, public exports, and import specifiers. */
export const ModuleDescription = defineSchema(schema.object({
    /** The module path relative to the package root. */
    path: PackagePath,
    /** The exact source bytes used for inspection. */
    source: PackageFile,
    /** The source length in UTF-16 code units. */
    length: schema.number().int().min(0),
    /** The declared dependency specifiers, including literal dynamic imports. */
    imports: schema.array(schema.string().min(1)),
    /** Execution APIs referenced by this source module. */
    globals: schema.array(GlobalReference),
    /** Symbols available to domain inspectors. */
    symbols: schema.array(SymbolDescription),
    /** Exports resolved to their original local or dependency symbols. */
    exports: schema.array(ExportDescription),
}));
/** A module description produced by a language inspector. */
export type ModuleDescription = schema.Infer<typeof ModuleDescription>;
