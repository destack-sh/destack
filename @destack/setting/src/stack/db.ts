import type { Table } from "@destack/db";
import { call, journal } from "@destack/object";
import { setting } from "../object/setting.ts";

/** The tables holding setting values beside their journal and call events. */
export const settingTables: readonly Table[] = [...setting.tables, journal, ...call.tables];
