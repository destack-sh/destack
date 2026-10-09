import type { Table } from "@destack/db";
import { integration, authorizationRequest, connection } from "../object/index.ts";

/** The connection kind's tables: its connections and pending authorizations. */
export const connectionTables: readonly Table[] = [
    ...connection.tables,
    ...authorizationRequest.tables,
];

/** The tables of integrations, which their owner's home space keeps and the spaces below copy. */
export const integrationTables: readonly Table[] = [...integration.tables];
