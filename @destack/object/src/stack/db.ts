import type { Table } from "@destack/db";
import { accessTables } from "@destack/access";
import { controllerLease } from "@destack/service/control";
import { outbox } from "@destack/service/outbox";
import { settlement } from "../method/settlement.ts";

/** The tables every durable object type's database holds beside its own: access, copies, settlements, controller leases and the outbox. */
export const serverTables: readonly Table[] = [
    ...accessTables,
    settlement,
    controllerLease,
    outbox,
];
