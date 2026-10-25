import type { Table } from "@destack/db";
import { integration, authorizationRequest, connection } from "../object/index.ts";

/** The connection kind's tables: its connections and pending authorizations. */
export const connectionTables: readonly Table[] = [
    ...connection.tables,
    ...authorizationRequest.tables,
];

/** The integration tables, stored at their primary and replicated by the spaces below. */
export const integrationTables: readonly Table[] = [...integration.tables];
