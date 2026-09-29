import type { Connector } from "@destack/resource";
import type { DatabaseConnection } from "../database/connection.ts";

// TODO #Incomplete: open databases on workerd through a connector of its own
/** The connectors opening databases on runtimes without a local one. */
export const connectors: Readonly<Record<string, Connector<DatabaseConnection>>> = {};
