import type { Table } from "@destack/db";
import { journal } from "@destack/audit";
import { reference, repository } from "../object/index.ts";

/** The repository service's tables: repositories, their references and its journal. */
export const repositoryTables: readonly Table[] = [
    ...repository.tables,
    ...reference.tables,
    journal,
];
