import { defineShape } from "@destack/object";
import { schema } from "@destack/schema";
import { integration } from "../object/integration.ts";
import { connection } from "../object/connection.ts";

/** The name of the shape replicating an integration to the spaces with its connections. */
const INTEGRATION_SHAPE = "integration";

/** The shape replicating a connection and its scope's integrations to a platform installation. */
const CONNECTION_SHAPE = "connection";

/** The integration a follower replicates. */
const IntegrationSelection = schema.object({
    /** The integration. */
    id: schema.identifier("integration"),
});

/** The connection a follower replicates. */
const ConnectionSelection = schema.object({
    /** The connection. */
    id: schema.identifier("connection"),
});

/** The shape replicating an integration, decided for the follower where the integration lives. */
export const integrationShape = defineShape({
    name: INTEGRATION_SHAPE,
    parameters: IntegrationSelection,
    objects: [integration],
    queries: ({ id }) => [integration.query({ where: { id } })],
});

/** Follow each integration of another scope a live connection refers to, from its primary. */
export const integrationFollow = connection.follow(integrationShape, {
    where: { deletedAt: { isNull: true } },
    key: async (row) => {
        // follow nothing for a connection without an integration, or with its own scope's
        if (row.integration === null || row.integration.scope === row.scope) {
            return undefined;
        }

        return {
            key: { scope: row.integration.scope, id: integration.identifier(row.integration.id) },
        };
    },
});

/** The shape replicating a connection the follower may use, and its scope's integrations. */
export const connectionShape = defineShape({
    name: CONNECTION_SHAPE,
    parameters: ConnectionSelection,
    objects: [connection, integration],
    queries: ({ id }) => [
        connection.query({ where: { id }, permission: "use" }),
        integration.query(),
    ],
});
