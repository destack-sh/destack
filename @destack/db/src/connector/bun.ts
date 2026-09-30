import type { Connector } from "@destack/resource";
import type { DatabaseConnection } from "../database/connection.ts";

/** The connectors opening databases on Bun: SQLite files, loaded on first connect. */
export const connectors: Readonly<Record<string, Connector<DatabaseConnection>>> = {
    sqlite: {
        code: "sqlite",
        connect: async (bound, declaration) => {
            const { sqliteConnector } = await import("../sqlite/connector.ts");

            return sqliteConnector.connect(bound, declaration);
        },
    },
};
