import { type Table, defineDatabase } from "@destack/db";
import { journal } from "@destack/audit";
import { account, connection, host, hostKey, zone } from "@destack/account/object";
import { dependency, packageObject, reference, release, repository, tag } from "../object/index.ts";

/** The forge's tables: repositories, references, packages, releases, tags, dependencies and its journal. */
export const forgeTables: readonly Table[] = [
    ...repository.tables,
    ...reference.tables,
    ...packageObject.tables,
    ...release.tables,
    ...tag.tables,
    ...dependency.tables,
    journal,
];

/** The forge's database, with copies of its residency's accounts, the connections it opens repositories through, the hosts and keys tokens name, and the zones standing for spaces. */
export const forgeDatabase = defineDatabase({
    name: "main",
    tables: forgeTables,
    copies: [account.table, connection.table, host.table, hostKey.table, zone.table],
});
