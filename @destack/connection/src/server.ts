import { principal } from "@destack/access";
import { implementPackage } from "@destack/object/server";
import { authorizationRequest } from "./object/authorization-request.ts";
import connectionPackage from "./package.ts";
import { implementConnection } from "./server/connection.ts";
import { implementIntegration } from "./server/integration.ts";
import { integrationFollow } from "./server/replica.ts";
import { TokenSource } from "./server/token.ts";

export * from "./server/index.ts";

/** The connection package's server: connections, integrations and other primaries' integrations. */
export default implementPackage(connectionPackage, (context) => {
    // source tokens as the installation, through the host's network and egress
    const tokens = new TokenSource({
        consumer: principal.installation.reference(context.scope, context.installation),
        fetch: (request) => context.fetch(request),
        at: (address) => context.at(address),
        environment: context.environment,
    });

    return {
        objects: [implementConnection(tokens), implementIntegration(tokens), authorizationRequest],
        follows: [integrationFollow],
    };
});
