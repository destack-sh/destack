import type { Table } from "@destack/db";
import { defineJournal } from "@destack/service/database";
import { setting } from "../object/setting.ts";

/** The requests the setting service executed. */
export const settingJournal = defineJournal("journal");

/** The tables holding setting values beside their journal. */
export const settingTables: readonly Table[] = [...setting.tables, settingJournal];
