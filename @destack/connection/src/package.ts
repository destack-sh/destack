import { definePackage } from "@destack/object";
import { ConnectionError } from "./error/error.ts";
import { authorizationRequest, connection, integration } from "./object/index.ts";

/** The name connection installs under in every space, as other installations address it. */
export const CONNECTION_INSTALLATION = "connection";

/** The handle spaces serve connections to external accounts through, and clients call them by. */
export default definePackage({
    objects: { integration, connection, authorizationRequest },
    errors: ConnectionError,
});
