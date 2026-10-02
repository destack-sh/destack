import type { Table } from "@destack/db";
import { journal } from "@destack/audit";
import { setting } from "../object/setting.ts";

/** The tables holding setting values beside their journal. */
export const settingTables: readonly Table[] = [...setting.tables, journal];
