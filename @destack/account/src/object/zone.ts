import { principal, relation, union } from "@destack/access";
import type { Select } from "@destack/db";
import { zoneTable } from "@destack/directory";
import { defineObject, INTRINSIC, method } from "@destack/object";
import { Scope } from "@destack/sync";

/** A space's zone as the directory places it: the cell serving it represents the space, and the cell a transfer moves it to reads it. */
export const zone = defineObject({
    name: "zone",
    plural: "zones",
    [INTRINSIC]: {
        table: zoneTable,
        mapping: {
            table: zoneTable,
            id: "id",
            scope: "scope",
            attributes: {},
            relations: {
                space: { column: "id", scope: Scope.universe.id },
                cell: { column: "cell", scope: Scope.universe.id },
                target: { column: "target", scope: Scope.universe.id },
            },
        },
    },
    scope: "universe",
    relations: {
        space: { subjects: [principal.space], grantedBy: null },
        cell: { subjects: [principal.cell], grantedBy: null },
        target: { subjects: [principal.cell], grantedBy: null },
    },
    permissions: {
        read: relation("target"),
        represent: union(relation("space"), relation("cell")),
    },
    methods: { get: method.get("read"), list: method.list("read") },
});
/** A zone as the directory places it. */
export type ZoneRow = Select<typeof zone.table>;
