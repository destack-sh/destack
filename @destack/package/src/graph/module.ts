import { defineSchema, Digest, schema } from "@destack/schema";
import { PackagePath } from "../file/file.ts";
import { Declaration } from "./declaration.ts";
import { Edge } from "./edge.ts";
import { Moniker } from "./moniker.ts";
import { Symbol } from "./symbol.ts";

/** A module's graph file: its symbols, declarations and outgoing edges. */
export const Module = Object.assign(
    defineSchema(
        schema.object({
            /** The package-relative module path. */
            path: PackagePath,
            /** The digest of the source bytes. */
            digest: Digest,
            /** The import specifiers, in source order. */
            imports: schema.array(schema.string().min(1)),
            /** The exports by name, each resolved to its original symbol. */
            exports: schema.array(
                schema.object({
                    /** The exported name. */
                    name: schema.string().min(1),
                    /** The original symbol. */
                    symbol: Moniker,
                    /** Whether the export exists only in the type system. */
                    isTypeOnly: schema.boolean(),
                }),
            ),
            /** The symbols and their members, by moniker. */
            symbols: schema.array(Symbol),
            /** The declarations, by moniker. */
            declarations: schema.array(Declaration),
            /** The edges from the module, its symbols and declarations, by source, kind and target. */
            edges: schema.array(Edge),
        }),
    ),
    { file },
);
/** A module's graph file. */
export type Module = schema.Infer<typeof Module>;

/** The index root of a build's graph: the graph file of each module. */
export const Root = defineSchema(
    schema.object({
        /** The digest of each module's graph file, by module path. */
        modules: schema.record(PackagePath, Digest),
    }),
);
/** The index root of a build's graph. */
export type Root = schema.Infer<typeof Root>;

/** Encode a module as its graph file: the bytes and their digest, the file's content address. */
async function file(module: Module): Promise<{ digest: Digest; bytes: Uint8Array<ArrayBuffer> }> {
    const bytes = new TextEncoder().encode(`${JSON.stringify(module)}\n`);

    return { digest: await Digest.of(bytes), bytes };
}
