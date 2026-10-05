import type { Table } from "@destack/db";
import { auditCall, auditTarget } from "../object/table.ts";

/** The audit history tables. */
export const auditTables: readonly Table[] = [auditCall, auditTarget];
