import { definePackage } from "@destack/object";
import { ConnectionError } from "./error/error.ts";
import { authorizationRequest, connection, integration } from "./object/index.ts";
import { connectionVault } from "./stack/vault.ts";
import { t } from "@destack/locale";

/** The name connection installs under in every space, as other installations address it. */
export const CONNECTION_INSTALLATION = "connection";

/** The capability through which connections reach the services they sign in to. */
const LEND_CAPABILITY = "lend";

/** The handle spaces serve connections to external accounts through, and clients call them by. */
export default definePackage({
    objects: [integration, connection, authorizationRequest],
    // the vault the connections keep their credentials in
    resources: [connectionVault],
    capabilities: [
        {
            name: LEND_CAPABILITY,
            reason: t`Sign in to the services you connect and call them for your installations`,
            sandbox: { network: { connect: ["*"] } },
        },
    ],
    errors: ConnectionError,
});
