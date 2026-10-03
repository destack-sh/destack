import type { Table } from "@destack/db";
import { journal } from "@destack/audit";
import { connection } from "@destack/account/object";
import { reference, repository } from "../object/index.ts";

/** The repository service's tables: repositories, their references, the copied account connections and its journal. */
export const repositoryTables: readonly Table[] = [
    ...repository.tables,
    ...reference.tables,
    connection.table,
    journal,
];
