import { defineProcedure, defineService } from "@destack/service";
import { schema } from "@destack/schema";
import { endpoint, message } from "../object/index.ts";

/** The pushes of a scope: the key its browsers subscribe with. */
export const push = {
    /** Read the VAPID public key browsers subscribe to a scope's pushes with (RFC 8292 3.2). */
    applicationServerKey: defineProcedure({
        authentication: "public",
        permission: "anyone",
        audit: false,
    })
        .route({ method: "GET", path: "/push/{scope}/key" })
        .input(schema.object({ /** The scope. */ scope: schema.string().min(1) }))
        .output(
            schema.object({
                /** The key, an uncompressed P-256 point in base64url. */
                key: schema.string(),
            }),
        ),
};

/** The key the messages of a scope are sealed to. */
export const key = {
    /** Read the public key a scope's messages are sealed to, its identity's message key. */
    read: defineProcedure({ authentication: "public", permission: "anyone", audit: false })
        .route({ method: "GET", path: "/keys/{scope}" })
        .input(schema.object({ /** The scope. */ scope: schema.string().min(1) }))
        .output(
            schema.object({
                /** The key, a P-256 point in base64. */
                key: schema.string(),
            }),
        ),
};

/** The message service: messages, endpoints and keys. */
export const messageService = defineService("message", {
    objects: { message, endpoint },
    push,
    key,
});
