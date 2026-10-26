import { none } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** The prompts a client may ask for (OIDC Core 3.1.2.1), one account per browser selecting none. */
export const Prompt = schema.enum(["none", "login", "consent"]);
/** A prompt a client may ask for. */
export type Prompt = schema.Infer<typeof Prompt>;

/** A validated authorization request a browser continues by reference (RFC 9126 request_uri). */
export const authorizationRequest = defineObject(
    {
        name: "authorization-request",
        plural: "authorizationRequests",
        scope: "universe",
        expiring: [{ after: { milliseconds: 0 }, from: "expiresAt" }],
        fields: {
            /** The client asking: an application's public client identifier or a device's. */
            clientId: field.string(),
            /** The registered redirect the code returns to. */
            redirectUri: field.string(schema.url()),
            /** The asked OAuth scopes. */
            scopes: field.json(schema.array(schema.string())),
            /** The client's opaque value, returned with the code or the error. */
            state: field.string().optional(),
            /** The client's value the ID token repeats (OIDC Core 3.1.2.1). */
            nonce: field.string().optional(),
            /** The S256 challenge of the client's code verifier (RFC 7636). */
            codeChallenge: field.string(),
            /** The package the tokens are for, as its resource (RFC 8707). */
            resource: field.string().optional(),
            /** The prompts the client asked for (OIDC Core 3.1.2.1). */
            prompts: field.json(schema.array(Prompt)),
            /** The longest age of the person's sign-in, in milliseconds (OIDC Core max_age). */
            maxAge: field.integer().optional(),
            /** When the request lapses. */
            expiresAt: field.time(),
        },
        permissions: { push: none() },
        constraints: { expiry: { kind: "index", on: ["expiresAt"] } },
    },
    (method) => ({
        /** Store a request for the browser to continue after sign-in or consent. */
        create: method.create("push", {
            fields: [
                "clientId",
                "redirectUri",
                "scopes",
                "state",
                "nonce",
                "codeChallenge",
                "resource",
                "prompts",
                "maxAge",
                "expiresAt",
            ],
            visibility: "package",
            isPredicted: false,
        }),
        /** Remove a request once the person decided on it. */
        delete: method.delete("push", { visibility: "package" }),
    }),
);
/** A stored authorization request. */
export type AuthorizationRequest = Select<typeof authorizationRequest.table>;
