import type { TableMapping } from "@destack/access";
import type { Table } from "@destack/db";
import { proposal, relationship, role } from "./access.ts";
import { auditEvent, auditTarget } from "./audit.ts";
import { activity, checkpoint } from "./history.ts";

/** The definition key holding an intrinsic object's table. */
export const INTRINSIC = Symbol("intrinsic");

/** The foreign table an object type maps, and how access finds relations there. */
export interface Intrinsic<Definition extends Table = Table> {
    /** The table holding the objects. */
    readonly table: Definition;
    /** Where access finds the objects. */
    readonly mapping?: Omit<TableMapping, "policy">;
}

/** Build the definitions of the intrinsic object types a scope type holds. */
export const Intrinsic = {
    role,
    relationship,
    proposal,
    auditEvent,
    auditTarget,
    activity,
    checkpoint,
};
