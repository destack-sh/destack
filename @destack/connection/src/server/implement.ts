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
    /** The space of the extension's connections, following other scopes' integration copies. */
    readonly space: Identifier<"space">;
}

/** The connection extension of a space, with the lender the machine's egress lends through. */
export interface ConnectionImplementation extends ObjectExtension {
    /** Lend the space's connections, applying their credentials to installations' calls. */
    lender(store: ObjectStore): Lender;
}

/** Implement the connection service. */
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
