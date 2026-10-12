import type { DatabaseConnector } from "../declare/database.ts";

/** The connectors opening databases on runtimes without a local one, such as browsers. */
export const connectors: Readonly<Record<string, DatabaseConnector>> = {};
