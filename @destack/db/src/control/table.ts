import { lock } from "../limit/lock.ts";
import type { Table } from "../table/table.ts";

/** The tables a loop keeps beside its controllers' on a shared database: their locks. */
export const CONTROL_TABLES: readonly Table[] = [lock];
