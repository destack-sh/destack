import { installedPackage, type TableMapping } from "@destack/access";
import type { Table } from "@destack/db";

/** Map packages onto a table of installations by package, each the set of its installations. */
export function mapInstalledPackages(table: Table): TableMapping {
    return {
        policy: installedPackage,
        table,
        id: "packageId",
        scope: "scope",
        attributes: {},
        relations: { installation: { column: "id" } },
    };
}
