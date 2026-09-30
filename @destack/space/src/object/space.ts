import { check, dialectSQL, sql, unique, type Select, type SQL } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { Risk } from "@destack/resource";
import { identifier, type Identifier, schema } from "@destack/schema";
import { replica, Scope } from "@destack/sync";
import { account, sudo } from "@destack/account/object";
import { SpaceName } from "@destack/host";

/** Move a space to another address within its account. */
const rename = method({
    permission: "update",
    input: schema.object({
        /** The new address. */
        name: SpaceName,
    }),
}).handle((call) => call.revise({ name: call.input.name }));

/** An isolated place for installations and their data, served by the cell of its zone. */
export const space = defineObject({
    name: "space",
    plural: "spaces",
    scope: account,
    isScope: true,
    controlled: true,
    fields: {
        /** The unique address within the account. */
        name: field.string(SpaceName),
        /** The display text, absent to show the name. */
        title: field.string(schema.string().min(1).max(128)).optional(),
        /** The least risk a resource plan waits for approval at. */
        approval: field.enum(Risk.options as [Risk, ...Risk[]]).default("backward-incompatible"),
    },
    constraints: (space) => [
        unique("space_scope_id").on(space.scope, space.id),
        check(
            "space_name",
            dialectSQL({
                sqlite: sql`length(${space.name}) BETWEEN 1 AND 63 AND ${space.name} NOT GLOB '*[^a-z0-9-]*' AND ${space.name} NOT LIKE '-%' AND ${space.name} NOT LIKE '%-' AND ${space.name} NOT LIKE '%--%'`,
                postgresql: sql`length(${space.name}) BETWEEN 1 AND 63 AND (${space.name} COLLATE "C") !~ '[^a-z0-9-]' AND ${space.name} NOT LIKE '-%' AND ${space.name} NOT LIKE '%-' AND ${space.name} NOT LIKE '%--%'`,
            }),
        ),
    ],
    indexes: { name: { on: ["name"], unique: true, across: account } },
    permissions: ["read", "list", "create", "update", "delete", "share"],
    shareable: { by: "share" },
    suspendable: { by: "update" },
    elevated: { delete: sudo },
    administration: ["read", "list", "update", "share"],
    methods: {
        get: method.get("read"),
        list: method.list("list"),
        create: method.create("create", { fields: ["name", "title"] }),
        update: method.update("update", { fields: ["title"] }),
        rename,
        delete: method.delete("delete"),
    },
});

/** A space, as its row in its cell's database records it. */
export type Space = Select<typeof space.table>;

/** The cell serving spaces: a host serving a device's own spaces, or a region whose hosts share its database. */
export type SpaceCell =
    | {
          /** The host. */
          readonly hostId: Identifier<"host">;
      }
    | {
          /** The region. */
          readonly regionId: Identifier<"region">;
      };

/** The cell serving spaces, as the directory's zones refer to it. */
export const SpaceCell = {
    /** Read the identifier the directory's zones refer to a cell by. */
    id(cell: SpaceCell): string {
        return "hostId" in cell ? cell.hostId : cell.regionId;
    },
    /** Read a cell from its identifier in the directory's zones: a host's or a region's. */
    parse(id: string): SpaceCell {
        const host = identifier("host").safeParse(id);

        return host.success ? { hostId: host.data } : { regionId: identifier("region").parse(id) };
    },
    /** Read the host a cell is, null for a region. */
    host(cell: SpaceCell): Identifier<"host"> | null {
        return "hostId" in cell ? cell.hostId : null;
    },
    /** Match the spaces this cell serves: those no transfer fences or copies. */
    served(): SQL {
        return sql`NOT EXISTS (SELECT 1 FROM ${Scope.table} WHERE ${Scope.table.scope} = ${space.table.id} AND ${Scope.table.fencedAt} IS NOT NULL) AND NOT EXISTS (SELECT 1 FROM ${replica} WHERE ${replica.scope} = ${space.table.id})`;
    },
};
