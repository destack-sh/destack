import type { Table } from "@destack/db";
import { accessTables } from "@destack/access";
import { controllerLease } from "@destack/service/control";
import { outbox } from "@destack/service/outbox";
import { settlement } from "../method/settlement.ts";

/** The tables of every durable type's database: access, settlements, leases and the outbox. */
export const storeTables: readonly Table[] = [...accessTables, settlement, controllerLease, outbox];
