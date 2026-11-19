import { type JsonObject, type JsonValue, schema } from "@destack/schema";

/** One object of a tree in depth-first order, with its last-sibling path. */
export interface TreeRow {
    /** The object. */
    readonly row: JsonObject;
    /** Whether the object and each ancestor below the root is the last of its siblings. */
    readonly isLast: readonly boolean[];
}

/** Objects nested under the object their parent field points to. */
const treeLayout = schema.object({
    /** The kind. */
    kind: schema.literal("tree"),
    /** The field with each object's identifier, which parents point to. */
    id: schema.string().min(1),
    /** The field with the parent's key. */
    parent: schema.string().min(1),
});
/** Objects nested under the object their parent field points to. */
export type TreeLayout = schema.Infer<typeof treeLayout>;

/** Objects nested under the object their parent field points to. */
export const TreeLayout = Object.assign(treeLayout, {
    /** Order objects depth first under their parents, one without a parent among them a root. */
    nest(layout: TreeLayout, rows: readonly JsonObject[]): TreeRow[] {
        // group the objects under their parents
        const keys = new Set(rows.map((row) => keyOf(row[layout.id])));
        const children = new Map<string, JsonObject[]>();
        const roots: JsonObject[] = [];
        for (const row of rows) {
            const parent = row[layout.parent];
            const isRoot = parent === null || parent === undefined || !keys.has(keyOf(parent));
            if (isRoot) {
                roots.push(row);
            } else {
                const siblings = children.get(keyOf(parent)) ?? [];
                siblings.push(row);
                children.set(keyOf(parent), siblings);
            }
        }

        // place each root's subtree
        const nested: TreeRow[] = [];
        for (const root of roots) {
            placeBelow(layout, root, [], children, nested);
        }

        return nested;
    },
});

/** Place an object and its subtree depth first below the levels above it. */
function placeBelow(
    layout: TreeLayout,
    row: JsonObject,
    isLast: readonly boolean[],
    children: ReadonlyMap<string, readonly JsonObject[]>,
    nested: TreeRow[],
): void {
    // place the object, then each child one level below it
    nested.push({ row, isLast });
    const below = children.get(keyOf(row[layout.id])) ?? [];
    for (const [index, child] of below.entries()) {
        placeBelow(layout, child, [...isLast, index === below.length - 1], children, nested);
    }
}

/** Read a key or parent value as the text the tree matches by. */
function keyOf(value: JsonValue | undefined): string {
    return typeof value === "string" || typeof value === "number"
        ? String(value)
        : JSON.stringify(value);
}
