import type { Table } from "@destack/db";
import { host, hostKey } from "../object/host.ts";
import { hostJournal } from "./journal.ts";

/** The host service's tables: hosts, their keys and its journal. */
export const hostTables: readonly Table[] = [...host.tables, ...hostKey.tables, hostJournal];
