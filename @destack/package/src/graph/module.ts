import type { Crypto } from "@destack/environment";
import { Digest, found, schema } from "@destack/schema";
import { PackagePath } from "../file/file.ts";
import { Declaration } from "./declaration.ts";
import { EDGE_KINDS, type Edge } from "./edge.ts";
import { Symbol } from "./symbol.ts";
import { Moniker } from "./moniker.ts";
import { PackageError } from "../error/index.ts";

/** An edge as stored: its two monikers' indexes among the module's monikers, and its relation. */
const StoredEdge = schema.tuple([
    schema.int().min(0),
    schema.int().min(0),
    schema.enum(EDGE_KINDS),
]);

/** A module's graph file, after SCIP's `Document`: its declarations, symbols and their edges. */
export const Module = Object.assign(
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
        /** The declarations, by moniker. */
        declarations: schema.array(Declaration),
        /** The symbols and their members, by moniker. */
        symbols: schema.array(Symbol),
        /** The monikers the edges relate, each once. */
        monikers: schema.array(Moniker),
        /** The edges from the module, its declarations and its symbols. */
        edges: schema.array(StoredEdge),
    }),
    { of, edges, file },
);
/** A module's graph file. */
export type Module = schema.Infer<typeof Module>;

/** Build a module's graph file from its edges, naming each moniker once. */
function of(
    fields: Omit<Module, "monikers" | "edges"> & { readonly edges: readonly Edge[] },
): Module {
    // number each moniker at its first use
    const indexes = new Map<Moniker, number>();
    for (const { from, to } of fields.edges) {
        for (const moniker of [from, to]) {
            if (!indexes.has(moniker)) {
                indexes.set(moniker, indexes.size);
            }
        }
    }

    // store each edge by its monikers' numbers
    const stored = fields.edges.map((edge): schema.Infer<typeof StoredEdge> => [
        found(indexes, edge.from),
        found(indexes, edge.to),
        edge.kind,
    ]);

    return { ...fields, monikers: [...indexes.keys()], edges: stored };
}

/** Read a module's edges by their monikers. */
function edges(module: Module): Edge[] {
    return module.edges.map(([from, to, kind]) => ({
        from: monikerAt(module, from),
        to: monikerAt(module, to),
        kind,
    }));
}

/** Read the moniker an edge names by its number, refusing one past the module's monikers. */
function monikerAt(module: Module, index: number): Moniker {
    const moniker = module.monikers[index];
    if (moniker === undefined) {
        throw PackageError.of(
            "INSPECTION_INVALID",
            `'${module.path}' is invalid: an edge names moniker '${String(index)}' of '${String(module.monikers.length)}'`,
        );
    }

    return moniker;
}

/** Encode a module as its graph file: the bytes and their digest, the file's content address. */
async function file(
    module: Module,
    crypto: Crypto,
): Promise<{ digest: Digest; bytes: Uint8Array<ArrayBuffer> }> {
    // refuse a declaration declared in another module
    const stray = module.declarations.find(
        (declaration) => Moniker.module(declaration.symbol) !== module.path,
    );
    if (stray !== undefined) {
        throw PackageError.of(
            "INSPECTION_INVALID",
            `'${module.path}' is invalid: it declares '${stray.moniker}', which belongs to another module`,
        );
    }

    // encode the module
    const bytes = new TextEncoder().encode(`${JSON.stringify(module)}\n`);

    return { digest: await Digest.of(bytes, crypto), bytes };
}
