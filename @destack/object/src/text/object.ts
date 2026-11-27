import { through } from "@destack/access";
import { argument, plural, t } from "@destack/locale";
import { method } from "../method/builder.ts";
import { type ObjectDefinition, type ObjectOf, ObjectType } from "../object/object.ts";
import { TEXT_READ, textChunkPolicy, textNodePolicy } from "./policy.ts";
import type { Policy } from "@destack/access";
import { CHUNKS, textChunkTable, textNodeTable } from "./table.ts";

import type {} from "@destack/package/import-meta";

/** The pieces of owners' texts, nested in any owner. */
export const textChunk = nestedInOwner({
    name: "text-chunk",
    plural: CHUNKS,
    title: t`${plural(argument("count"), { one: "Text chunk", other: "Text chunks" })}`,
    table: textChunkTable,
    policy: textChunkPolicy,
});

/** The nodes of owners' texts, one row per field and node, nested in any owner. */
export const textNode = nestedInOwner({
    name: "text-node",
    plural: "textNodes",
    title: t`${plural(argument("count"), { one: "Text node", other: "Text nodes" })}`,
    table: textNodeTable,
    policy: textNodePolicy,
});

/** Declare a type of text rows nested in an owner of any type, which its text readers read. */
function nestedInOwner<Rows extends typeof textChunkTable | typeof textNodeTable>(declared: {
    readonly name: string;
    readonly plural: string;
    readonly title: ObjectDefinition["title"];
    readonly table: Rows;
    readonly policy: Policy;
}): ObjectOf<{ table: Rows; fields: {} }> {
    // store the rows in the given table, their parent relation open to any owner
    const { table, policy, ...named } = declared;
    const methods = { list: method.list("read") };
    const definition: ObjectDefinition = {
        ...named,
        represents: policy,
        nested: { in: "any", add: TEXT_READ },
        relations: { parent: { subjects: [], grantedBy: null, open: true } },
        permissions: { read: through("parent", TEXT_READ) },
        methods,
        mapping: {
            table,
            id: "id",
            scope: "scope",
            attributes: {},
            relations: {
                parent: {
                    column: "parentId",
                    subject: { packageId: "parentPackageId", type: "parentType", scope: "scope" },
                },
            },
        },
    };

    // type the rows by their table, which the type stores them in
    const assembled = ObjectType.assemble(import.meta.destack.package, {
        ...definition,
        table,
        methods,
        source: definition,
    });
    if (!isOver(assembled, table)) {
        throw new TypeError(`type '${declared.name}' is not over its table`);
    }

    return assembled;
}

/** Report whether a type stores its rows in a table, without fields of its own. */
function isOver<Rows extends typeof textChunkTable | typeof textNodeTable>(
    type: ObjectType,
    table: Rows,
): type is ObjectOf<{ table: Rows; fields: {} }> {
    return type.table === table;
}
