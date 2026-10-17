import type { Table } from "@destack/db";
import {
    alert,
    alertRule,
    issue,
    issuePerson,
    respond,
    silence,
    visitorSalt,
} from "../object/index.ts";

/** The tables of a space's durable observability types and its responses' runs. */
export const observabilityTables: readonly Table[] = [
    issue,
    issuePerson,
    alertRule,
    alert,
    silence,
    visitorSalt,
    ...Object.values(respond.objects),
].flatMap((object) => object.tables);
