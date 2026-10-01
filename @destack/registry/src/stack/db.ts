import type { Table } from "@destack/db";
import { journal } from "@destack/audit";
import { dependency, packageObject, release, tag } from "../object/index.ts";

/** The registry's tables: packages, releases, tags, dependencies and its journal. */
export const registryTables: readonly Table[] = [
    ...packageObject.tables,
    ...release.tables,
    ...tag.tables,
    ...dependency.tables,
    journal,
];
