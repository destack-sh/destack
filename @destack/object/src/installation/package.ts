import { Policy, principal, type TableMapping } from "@destack/access";
import type { Table } from "@destack/db";
import { Package } from "@destack/package";
import type {} from "@destack/package/import-meta";

/** Packages as the sets of their installations in a scope, which attachment hosts let attach. */
export const installedPackage = new Policy(Package.of(import.meta.destack.package), {
    name: "package",
    relations: { installation: { subjects: [principal.installation] } },
    permissions: {},
});

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
