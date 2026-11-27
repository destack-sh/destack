import { Policy, through } from "@destack/access";

import type {} from "@destack/package/import-meta";

/** The owner permission whose holders read its text. */
export const TEXT_READ = "text";

/** The access of a text's chunks: nested in an owner of any type, read by its text readers. */
export const textChunkPolicy = textPolicy("text-chunk");

/** The access of a text's nodes: nested in an owner of any type, read by its text readers. */
export const textNodePolicy = textPolicy("text-node");

/** Declare the access of a type of text rows, whose owners join its open parent relation. */
function textPolicy(name: string): Policy {
    return new Policy(import.meta.destack.package, {
        name,
        relations: { parent: { subjects: [], grantedBy: null, open: true } },
        permissions: { read: through("parent", TEXT_READ) },
    });
}
