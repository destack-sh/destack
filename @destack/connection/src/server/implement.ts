import type { Identifier } from "@destack/schema";
import type { ObjectExtension, ObjectStore } from "@destack/object/server";
import { connection } from "../object/connection.ts";
import { connectionService } from "../service/index.ts";
import { serveIntegrations } from "./integration.ts";
import { IntegrationCopy, integrationShape, connectionShape } from "./copy.ts";
import { Lender } from "./lender.ts";
import { serveConnections } from "./server.ts";
import { TokenSource, type TokenSourceOptions } from "./token.ts";

/** What a machine serves a space's connections with: the token source's options and the space. */
export interface ConnectionOptions extends TokenSourceOptions {
    /** The space whose connections the extension keeps, which follows the copies of other scopes' integrations. */
    readonly space: Identifier<"space">;
}

/** The connection extension of a space, with the lender the machine's egress lends its connections through. */
export interface ConnectionImplementation extends ObjectExtension {
    /** Lend the space's connections through its store, applying their credentials to its installations' outbound calls. */
    lender(store: Omit<ObjectStore, "router">): Lender;
}

/** Implement the connection service: keep a space's connections, their credentials current, and lend them to the machine's egress; keep the space's integrations where it has no home, else copy them from it. */
export function implementConnections(options: ConnectionOptions): ConnectionImplementation {
    // request the tokens of the integrations the space keeps, or of copies through their home
    const { home, space, ...rest } = options;
    const tokens = new TokenSource({ ...rest, ...(home === undefined ? {} : { home }) });
    const connections = serveConnections(tokens);

    return {
        service: connectionService,
        objects:
            home === undefined
                ? { ...connections, integration: serveIntegrations(tokens) }
                : connections,
        shapes: [integrationShape, connectionShape],
        watches: [connection.table],
        ...(home === undefined
            ? {}
            : {
                  // copy the integrations of other scopes the connections name, from their home
                  subscriptions: (objects) =>
                      IntegrationCopy.subscriptions(objects.database, space, home),
              }),
        lender: (store) => new Lender(store, tokens),
    };
}
