import type { Crypto } from "@destack/environment";
import { Digest, schema } from "@destack/schema";
import { PackagePath } from "../file/file.ts";
import { Declaration } from "./declaration.ts";
import { Edge, MODULE_EDGE_KINDS } from "./edge.ts";
import { Moniker } from "./moniker.ts";
import { PackageError } from "../error/index.ts";

/** A module's graph file: its imports, exports, declarations and their edges. */
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
        /** The edges from the module and its declarations, every declaration edge but covers. */
        edges: schema.array(Edge.extend({ kind: schema.enum(MODULE_EDGE_KINDS) })),
    }),
    { file },
);
/** A module's graph file. */
export type Module = schema.Infer<typeof Module>;

/** The index root of a build's graph: the graph file of each module. */
export const Root = schema.object({
    /** The digest of each module's graph file, by module path. */
    modules: schema.record(PackagePath, Digest),
});
/** The index root of a build's graph. */
export type Root = schema.Infer<typeof Root>;

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
            `${module.path} declares ${stray.moniker}, which belongs to another module`,
        );
    }

    // encode the module
    const bytes = new TextEncoder().encode(`${JSON.stringify(module)}\n`);

    return { digest: await Digest.of(bytes, crypto), bytes };
}
