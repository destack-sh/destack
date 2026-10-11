import { and, eq } from "@destack/db";
import {
    type CallOf,
    type ObjectReconciliation,
    observeCondition,
    type PreparedCallOf,
    type ResultOf,
} from "@destack/object";
import { aligned, present } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { ConnectionError } from "../error/error.ts";
import { authorizationRequest } from "../object/authorization-request.ts";
import type { Token } from "../credential/credential.ts";
import {
    type Connection,
    type ConnectionKind,
    connection,
    Withdrawal,
} from "../object/connection.ts";
import {
    PERSON_KINDS,
    REFRESH_MARGIN,
    requireSecret,
    SUPPLIED_KINDS,
    TokenSource,
} from "./token.ts";
import { requireVault } from "./integration.ts";
import { complete, exchange, poll, start, started } from "./authorization.ts";
import { ConsumerSecret } from "@destack/vault/server";
import type { ObjectStore } from "@destack/object/server";

/** How long a connection waits after its authorization server failed to answer before trying again: a minute. */
const RETRY_MILLISECONDS = 60_000;

/** Serve connections: credentials kept in vaults as the service, authorized by people, kept current by the controller beside the lender, and issued to the spaces lending them to bound installations. */
export function serveConnections(tokens: TokenSource) {
    return {
        connection: connection.implement({
            methods: {
                create: {
                    authorize: async (call) => {
                        requireHome(call.input.protocol.kind, tokens);
                        requireClient(call.input);
                        requireVault(call.input.vault, call.scope);
                    },
                    prepare: (call) => keep(call, tokens),
                    handle: (call, next) => next(),
                },
                authorize: {
                    authorize: async (call) => requireOpen(call.requireTarget()),
                    prepare: (call) => start(call, tokens),
                    handle: (call) => started(call),
                },
                complete: {
                    prepare: (call) => exchange(call, tokens),
                    handle: (call) => complete(call),
                },
                revoke: {
                    authorize: async (call) => requireOpen(call.requireTarget()),
                    prepare: async (call) => withdrawal(call.requireTarget()),
                    handle: (call) => revoke(call),
                    settle: async (call, settled) => {
                        // withdraw the tokens once the revocation committed
                        if (settled.status === "committed") {
                            await withdraw(call.requireStore(), settled.prepared, tokens);
                        }
                    },
                },
                requestToken: {
                    prepare: (call) =>
                        lend(call.requireTarget(), tokens, call.requireStore(), call.now),
                    handle: async (call) => call.prepared,
                },
            },
            controller: {
                pending: {},
                reconcile: (reconciliation) => reconcile(reconciliation, tokens),
            },
        }),
        authorizationRequest,
    };
}

/** Keep the material a connection's kind sends in its vault before the connection is created. */
async function keep(call: CallOf<typeof connection, "create">, tokens: TokenSource) {
    // require material of the connection's kind for exactly the kinds that send it
    const { protocol, vault, material } = call.input;
    const isSupplied = SUPPLIED_KINDS.has(protocol.kind) || protocol.kind === "jwt";
    if (
        isSupplied !== (material !== undefined) ||
        (material !== undefined && material.kind !== protocol.kind)
    ) {
        throw new ServiceError("BAD_REQUEST", {
            message: isSupplied
                ? `a ${protocol.kind} connection is created with its ${protocol.kind} material`
                : `a ${protocol.kind} connection is authorized, never created with material`,
        });
    }

    // keep the material
    if (material !== undefined) {
        const id = present(call.id, "a prepared creation's identifier");
        await tokens.write(call.requireStore(), { id, vault }, { material });
    }

    return {};
}

/** Describe the credential a revocation withdraws, without the credential itself. */
function withdrawal(target: Connection): Withdrawal {
    return {
        id: target.id,
        protocol: target.protocol,
        vault: target.vault,
        integration: target.integration,
        subject: target.subject,
    };
}

/** Mark a connection revoked. */
async function revoke(
    call: PreparedCallOf<typeof connection, "revoke">,
): Promise<ResultOf<typeof connection, "revoke">> {
    const target = call.requireTarget();

    return call.updateStatus({
        expiresAt: null,
        conditions: {
            ...target.conditions,
            Authorized: observeCondition(
                target.conditions["Authorized"],
                {
                    status: "false",
                    reason: "Revoked",
                    message: "revoked",
                    observedGeneration: target.generation,
                },
                call.now,
            ),
        },
    });
}

/** Revoke a committed revocation's tokens at their server, or uninstall its integration, then destroy its credential. */
async function withdraw(
    system: ObjectStore,
    prepared: unknown,
    tokens: TokenSource,
): Promise<void> {
    // read the credential
    const withdrawn = Withdrawal.parse(prepared);
    const credential = await tokens.read(system, withdrawn);
    const token = credential.token;

    // uninstall an installed integration with its private key
    if (withdrawn.protocol.kind === "installation" && withdrawn.subject !== null) {
        const client = await tokens.integration(system, withdrawn.integration);
        const issuer = tokens.issuer(client.integration);
        await issuer.revoke(client.integration.clientId, withdrawn.subject, requireSecret(client));
    }
    // revoke the refresh token, or else the access token, at the authorization server
    else if (token !== undefined && withdrawn.integration !== null) {
        const { server } = await tokens.integration(system, withdrawn.integration);
        await (token.refreshToken === undefined
            ? server.revoke(token.accessToken, "access_token")
            : server.revoke(token.refreshToken, "refresh_token"));
    }

    // destroy the credential
    const secret = TokenSource.secret(withdrawn.id);
    await ConsumerSecret.destroy(system, withdrawn.vault, secret, tokens.options.consumer);
}

/** Keep a connection's credential current: mark supplied material authorized, poll a device authorization, and obtain or refresh tokens before they expire. */
async function reconcile(
    reconciliation: ObjectReconciliation<typeof connection>,
    tokens: TokenSource,
): Promise<number | undefined> {
    // read the key's one connection at its current revision
    const { now } = reconciliation;
    const system = reconciliation.store;
    const [read] = await reconciliation.database
        .select()
        .from(connection.table)
        .where(eq(connection.table.id, aligned(reconciliation.rows, 0).id));
    if (read === undefined) {
        return undefined;
    }
    const current = read;
    const authorized = current.conditions["Authorized"];
    const kind = current.protocol.kind;

    // destroy a deleted connection's credential before it goes
    if (current.deletionRequestedAt !== null) {
        await finalize(reconciliation, current, tokens);

        return undefined;
    }
    // leave a revoked connection
    else if (authorized?.reason === "Revoked") {
        return undefined;
    }
    // mark supplied material authorized
    else if (SUPPLIED_KINDS.has(kind)) {
        if (authorized?.status !== "true") {
            const supplied = { status: "true" as const, reason: "Supplied", message: "" };
            await tokens.observe(system, current, supplied, {}, now);
        }

        return undefined;
    }
    // poll a device authorization or wait for a person to authorize the others
    else if (PERSON_KINDS.has(kind) && authorized?.status !== "true") {
        return awaitPerson(reconciliation, current, tokens);
    }
    // obtain or refresh a token before it expires
    else if (authorized?.reason !== "Expired") {
        return keepToken(system, current, tokens, now);
    }
    // leave an expired grant for a person to authorize again
    else {
        return undefined;
    }
}

/** Destroy a deleted connection's credential, then let it go. */
async function finalize(
    reconciliation: ObjectReconciliation<typeof connection>,
    current: Connection,
    tokens: TokenSource,
): Promise<void> {
    await ConsumerSecret.destroy(
        reconciliation.store,
        current.vault,
        TokenSource.secret(current.id),
        tokens.options.consumer,
    );
    await reconciliation.mutate([connection.call("finalize", connection.target(current))]);
}

/** Poll a connection's device authorization, or mark it pending until a person authorizes it, returning when to look again. */
async function awaitPerson(
    reconciliation: ObjectReconciliation<typeof connection>,
    current: Connection,
    tokens: TokenSource,
): Promise<number | undefined> {
    // poll a pending device authorization
    const [request] = await reconciliation.database
        .select()
        .from(authorizationRequest.table)
        .where(
            and(
                eq(authorizationRequest.table.parentId, current.id),
                eq(authorizationRequest.table.kind, "device"),
            ),
        );
    if (request !== undefined) {
        return poll(reconciliation, current, request, tokens);
    }

    // mark the connection pending until a person authorizes it
    if (current.conditions["Authorized"] === undefined) {
        const message = "waiting for a person to authorize the connection";
        const pending = { status: "false" as const, reason: "Pending", message };
        await tokens.observe(reconciliation.store, current, pending, {}, reconciliation.now);
    }

    return undefined;
}

/** Obtain a connection's token, or refresh it before it expires, returning when to look again. */
async function keepToken(
    system: ObjectStore,
    current: Connection,
    tokens: TokenSource,
    now: number,
): Promise<number | undefined> {
    // keep an authorized token that never expires
    const authorized = current.conditions["Authorized"];
    const isAuthorized = authorized?.status === "true";
    if (isAuthorized && current.expiresAt === null) {
        return undefined;
    }

    // wait until its refresh is due, then obtain or refresh it
    const due =
        isAuthorized && current.expiresAt !== null ? current.expiresAt - REFRESH_MARGIN : now;
    if (due > now) {
        return due - now;
    }

    return refresh(system, current, tokens, now);
}

/** Refresh a connection's token, recording a failure other than a refused grant and waiting before the next try. */
async function refresh(
    system: ObjectStore,
    row: Connection,
    tokens: TokenSource,
    now: number,
): Promise<number | undefined> {
    try {
        const { token } = await tokens.refresh(system, row, now);
        const expiresAt = token?.expiresAt;

        return expiresAt === undefined ? undefined : Math.max(0, expiresAt - REFRESH_MARGIN - now);
    } catch (error) {
        // leave an expired grant for a person and retry other failures
        if (!(error instanceof ConnectionError)) {
            throw error;
        } else if (error.error !== "invalid_grant") {
            await tokens.observe(
                system,
                row,
                { status: "false", reason: "Failing", message: error.message },
                {},
                now,
            );

            return RETRY_MILLISECONDS;
        }

        return undefined;
    }
}

/** Answer a connection's current access token for a space lending it, refusing a credential lent only where it is kept. */
async function lend(
    row: Connection,
    tokens: TokenSource,
    system: ObjectStore,
    now: number,
): Promise<Token> {
    // require an authorized connection holding a token
    if (row.conditions["Authorized"]?.status !== "true") {
        throw new ConnectionError("UNAUTHORIZED", `connection ${row.id} is not authorized`);
    }
    const { token } = await tokens.current(system, row, now);
    if (token === undefined) {
        throw new ConnectionError(
            "UNAUTHORIZED",
            `a ${row.protocol.kind} connection lends its material only where it is kept`,
        );
    }

    return token;
}

/** Refuse an installation connection where its integration is a copy, as installations issue their tokens with the integration's key at its home. */
function requireHome(kind: ConnectionKind, tokens: TokenSource): void {
    if (kind === "installation" && tokens.options.home !== undefined) {
        throw new ServiceError("BAD_REQUEST", {
            message: "installation connections live at their integration's home",
        });
    }
}

/** Refuse a connection lacking the client its kind authorizes through: an integration, or for the authorization code grant a resource whose server registers one. */
function requireClient(input: CallOf<typeof connection, "create">["input"]): void {
    // read whether the kind authorizes through a client and whether the connection has one
    const { protocol, integration, resource } = input;
    const hasClient =
        integration !== undefined ||
        (protocol.kind === "authorization-code" && resource !== undefined);

    // refuse a missing client
    if (!SUPPLIED_KINDS.has(protocol.kind) && !hasClient) {
        throw new ServiceError("BAD_REQUEST", {
            message:
                protocol.kind === "authorization-code"
                    ? "an authorization-code connection needs an integration, or a resource whose server registers its client"
                    : `a ${protocol.kind} connection needs its integration`,
        });
    }
}

/** Refuse to authorize or revoke a revoked connection. */
function requireOpen(target: Connection): void {
    if (target.conditions["Authorized"]?.reason === "Revoked") {
        throw new ServiceError("CONFLICT", { message: `connection ${target.id} is revoked` });
    }
}
