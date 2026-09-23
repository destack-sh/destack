import { identifier, schema } from "@destack/schema";
import { defineProcedure, eventIterator } from "@destack/service";
import { Creation } from "@destack/service/procedure";
import { PackageId } from "@destack/package";
import definition from "../../destack.json" with { type: "json" };
import { defineOperation } from "@destack/service/operation";
import { Subject } from "@destack/access";

/** A signed-in identity retained privately by this daemon. */
export const Login = schema.object({
    /** Local selection identifier, never a bearer credential. */
    id: identifier("login"),
    /** Authentication issuer used to establish and refresh the session. */
    issuer: schema.httpUrl(),
    /** Last known session at the issuer; renewal may replace it without replacing this login. */
    sessionId: identifier("session").nullable(),
    /** Authority-qualified user; account memberships are separate. */
    subject: Subject.extend({ kind: schema.literal("user"), id: identifier("user") }),
    /** Last verified display name. */
    name: schema.string(),
    /** Whether this login can obtain credentials without another sign-in. */
    status: schema.enum(["authenticated", "reauthentication"]),
    /** Last observed authority reachability, independent of authentication. */
    connection: schema.enum(["unknown", "reachable", "unreachable"]),
    /** Last successful authority contact, in UTC milliseconds. */
    verifiedAt: schema.number().int().nullable(),
});

/** A signed-in identity retained privately by this daemon. */
export type Login = schema.Infer<typeof Login>;

/** Browser sign-in progress shared by CLI and Home. */
export const SignInOperation = defineOperation(
    Login,
    schema.object({
        /** Selected authentication issuer. */
        issuer: schema.httpUrl(),
        /** Exclusive operation expiry, in UTC milliseconds. */
        expiresAt: schema.number().int(),
        /** Browser authorization URL, containing no session credential. */
        url: schema.httpUrl(),
    }),
);

/** Select a retained sign-in operation. */
const signInKey = schema.object({ id: schema.uuid() });

/** Manage concurrent signed-in identities without a process-wide active session. */
export const login = {
    /** Read one retained identity without disclosing credentials. */
    get: access("read")
        .route({ method: "GET", path: "/logins/{loginId}" })
        .input(schema.object({ loginId: identifier("login") }))
        .output(Login),
    /** List the identities available to local clients. */
    list: access("read").route({ method: "GET", path: "/logins" }).output(schema.array(Login)),
    /** Send an initial complete list and replace it after changes; cursors are unnecessary. */
    watch: access("read")
        .route({ method: "GET", path: "/logins/watch" })
        .output(eventIterator(schema.array(Login))),
    /** Remove local credentials even when remote revocation cannot be confirmed. */
    signOut: access("sign-out")
        .route({ method: "POST", path: "/logins/{loginId}/sign-out" })
        .input(Creation.extend({ loginId: identifier("login") }))
        .output(
            schema.object({
                /** Whether the authority confirmed revocation before local credentials were removed. */
                revocation: schema.enum(["confirmed", "unconfirmed"]),
            }),
        ),
    /** Complete authority-managed authentication in a browser. */
    signIn: {
        start: access("sign-in")
            .route({ method: "POST", path: "/sign-ins" })
            .input(
                Creation.extend({
                    /** Issuer selected explicitly by the user or device configuration. */
                    issuer: schema.httpUrl(),
                    /** Existing login to reauthenticate without replacing other logins. */
                    loginId: identifier("login").optional(),
                }),
            )
            .output(SignInOperation.operation),
        get: access("read")
            .route({ method: "GET", path: "/sign-ins/{id}" })
            .input(signInKey)
            .output(SignInOperation.operation),
        watch: access("read")
            .route({ method: "GET", path: "/sign-ins/{id}/watch" })
            .input(signInKey)
            .output(eventIterator(SignInOperation.operation)),
        cancel: access("sign-in")
            .route({ method: "POST", path: "/sign-ins/{id}/cancel" })
            .input(signInKey.extend(Creation.shape))
            .output(SignInOperation.operation),
    },
};

/** Restrict credential administration to authorized local clients. */
function access(action: "read" | "sign-in" | "sign-out") {
    return defineProcedure({
        authentication: "host",
        permission: { packageId: PackageId.parse(definition.id), type: "login", name: action },
        audit: action !== "read",
    });
}
