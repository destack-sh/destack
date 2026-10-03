import type { DatabaseConnector } from "../declare/database.ts";

// TODO #Incomplete: open databases on workerd through a workerd connector
/** The connectors opening databases on runtimes without a local one. */
export const connectors: Readonly<Record<string, DatabaseConnector>> = {};
