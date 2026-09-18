import { defineSchema, schema } from "@destack/schema";
import { PackagePath } from "../file/file.ts";
import { SymbolReference } from "./reference.ts";
import { SymbolDescription } from "./symbol.ts";
import { PackageFile } from "../file/file.ts";

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
    /** Symbols available to domain inspectors. */
    symbols: schema.array(SymbolDescription),
    /** Exports resolved to their original local or dependency symbols. */
    exports: schema.array(ExportDescription),
}));
/** A module description produced by a language inspector. */
export type ModuleDescription = schema.Infer<typeof ModuleDescription>;
