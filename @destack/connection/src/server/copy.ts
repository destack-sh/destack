import type { ServiceClient } from "@destack/service/client";
import type { connectionService } from "../service/service.ts";
import {
    and,
    type Condition,
    type DatabaseConnection,
    eq,
    isNotNull,
    isNull,
    type Table,
} from "@destack/db";
import type { SubscriberEntry } from "@destack/object/server";
import { schema } from "@destack/schema";
import {
    defineShape,
    type ObjectReference,
    Replica,
    replica,
    type Subject,
    type Subscription,
    Uplink,
} from "@destack/sync";
import { integration } from "../object/integration.ts";
import { connection } from "../object/connection.ts";

/** The name of the shape copying an integration to the spaces holding its connections. */
const INTEGRATION_SHAPE = "integration";

/** The name of the shape copying a connection and its scope's integrations to a platform installation opening repositories through it. */
const CONNECTION_SHAPE = "connection";

/** The integration a follower copies. */
const IntegrationSelection = schema.object({
    /** The integration. */
    id: schema.identifier("integration"),
});

/** The connection a follower copies. */
const ConnectionSelection = schema.object({
    /** The connection. */
    id: schema.identifier("connection"),
});

/** The shape copying an integration, decided for the follower where the integration lives. */
export const integrationShape = defineShape({
    name: INTEGRATION_SHAPE,
    parameters: IntegrationSelection,
    audience: "reader",
    isReadinessGate: true,
    replica: ({ name, scope, parameters }) =>
        new Replica({
            name,
            scope,
            tables: [integration.table],
            where: new Map<Table, Condition>([[integration.table, { id: parameters.id }]]),
        }),
});

/** The copies a space keeps of the integrations its connections authorize at, from their home. */
export const IntegrationCopy = {
    /** Write the name of a follower's copy of an integration, one per integration. */
    copy(id: string): string {
        return `${INTEGRATION_SHAPE}:${id}`;
    },

    /** Report whether the database keeps an integration as a copy from its home. */
    async isCopied(database: DatabaseConnection, reference: ObjectReference): Promise<boolean> {
        const [copied] = await database
            .select({ name: replica.name })
            .from(replica)
            .where(
                and(
                    eq(replica.name, IntegrationCopy.copy(reference.id)),
                    eq(replica.scope, reference.scope),
                ),
            );

        return copied !== undefined;
    },

    /** List the copies of the integrations of other scopes the database's connections authorize at, followed as its space. */
    async subscriptions(
        database: DatabaseConnection,
        space: string,
        home: (scope: string, signer?: Subject) => ServiceClient<typeof connectionService>,
    ): Promise<readonly SubscriberEntry[]> {
        // read the live connections referencing an integration of another scope
        const rows = await database
            .select({ scope: connection.table.scope, integration: connection.table.integration })
            .from(connection.table)
            .where(
                and(
                    isNotNull(connection.table.integration),
                    isNull(connection.table.deletionRequestedAt),
                ),
            );
        const referenced = new Map(
            rows.flatMap(({ scope, integration: named }) =>
                named === null || named.scope === scope
                    ? []
                    : [[`${named.scope}/${named.id}`, named] as const],
            ),
        );

        // follow each integration once
        return [...referenced.values()].map((named) => {
            return {
                subscription: integrationShape.subscription({
                    name: IntegrationCopy.copy(named.id),
                    scope: named.scope,
                    below: space,
                    parameters: { id: schema.identifier("integration").parse(named.id) },
                }),
                publisher: Uplink.of((signer) => home(named.scope, signer).replica),
            };
        });
    },
};

/** The shape copying one connection and the integrations of its scope, decided for the calling follower where they live. */
export const connectionShape = defineShape({
    name: CONNECTION_SHAPE,
    parameters: ConnectionSelection,
    audience: "caller",
    isReadinessGate: true,
    replica: ({ name, scope, parameters }) =>
        new Replica({
            name,
            scope,
            tables: [connection.table, integration.table],
            where: new Map<Table, Condition>([[connection.table, { id: parameters.id }]]),
        }),
});

/** The copies of single connections a platform installation keeps, such as the forge's of the connections its repositories reference. */
export const ConnectionCopy = {
    /** Write the name of a follower's copy of a connection, one per connection. */
    copy(id: string): string {
        return `${CONNECTION_SHAPE}:${id}`;
    },

    /** Request the copy of a connection of a scope for a follower. */
    subscription(below: string, scope: string, id: string): Subscription {
        return connectionShape.subscription({
            name: ConnectionCopy.copy(id),
            scope,
            below,
            parameters: { id: schema.identifier("connection").parse(id) },
        });
    },
};
