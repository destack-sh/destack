import { and, eq } from "@destack/db";
import {
    type MethodContextOf,
    type ObjectReconciliation,
    type ResultOf,
    StatusCondition,
} from "@destack/object";
import { RetryPolicy } from "@destack/environment";
import { nth, present } from "@destack/schema";
import { ConnectionError } from "../error/error.ts";
import { authorizationRequest } from "../object/authorization-request.ts";
import { Credential } from "../credential/credential.ts";
import {
    Connection,
    type ConnectionKind,
    connection,
    type AuthorizedReason,
    type Lease,
    REVOKED,
} from "../object/connection.ts";
import { PERSON_KINDS, REFRESH_MARGIN, SUPPLIED_KINDS, TokenSource, Runner } from "./token.ts";
import { ConnectionAuthorization } from "./authorization.ts";

/** How failed refreshes retry: 5 s doubling to 15 min with full jitter, as message's sends do. */
const RETRY = RetryPolicy.of({
    initialInterval: 5_000,
    maximumInterval: 15 * 60_000,
    jitter: "full",
});

/** Implement connections. */
export function implementConnection(tokens: TokenSource): typeof connection {
    const authorization = new ConnectionAuthorization(tokens);

    return connection.implement({
        methods: {
            create: {
                authorize: async (context) => {
                    // require the space, client, vault and exchange source the connection's kind needs
                    requireInstallable(context.input.protocol.kind, context.chain, tokens);
                    requireClient(context.input);
                    Credential.requireVault(context.input.vault, context.scope);
                    await Connection.requireSource(context);
                },
                prepare: (context) => storeMaterial(context, tokens),
                handle: (context, next) =>
                    next(
                        context.with({
                            input: {
                                ...context.input,
                                storedAt: context.input.material === undefined ? null : context.now,
                            },
                        }),
                    ),
            },
            lease: {
                prepare: (context) => lease(context.target, tokens, context),
                handle: async (context) => context.prepared,
            },
            authorize: {
                authorize: async (context) => requireOpen(context.target),
                prepare: (context) => authorization.start(context),
                handle: (context) => authorization.started(context),
            },
            complete: {
                prepare: (context) => authorization.exchange(context),
                handle: (context) => authorization.complete(context),
            },
            revoke: {
                authorize: async (context) => requireOpen(context.target),
                handle: (context) => revoke(context),
            },
        },
        controller: {
            pending: {},
            reconcile: (reconciliation) => reconcile(reconciliation, tokens, authorization),
        },
    });
}

/** Store the material a connection's kind sends in its vault before the connection is created. */
async function storeMaterial(
    context: MethodContextOf<typeof connection, "create", unknown>,
    tokens: TokenSource,
) {
    // require material of the connection's kind for exactly the kinds that send it
    const { protocol, vault, material } = context.input;
    const isSupplied = SUPPLIED_KINDS.has(protocol.kind) || protocol.kind === "jwt";
    if (
        isSupplied !== (material !== undefined) ||
        (material !== undefined && material.kind !== protocol.kind)
    ) {
        throw ConnectionError.of(
            "MATERIAL_MISMATCH",
            isSupplied
                ? `a ${protocol.kind} connection is created with its ${protocol.kind} material`
                : `a ${protocol.kind} connection is authorized, never created with material`,
        );
    }

    // store the material
    if (material !== undefined) {
        const id = present(context.id, "a prepared creation's identifier");
        await tokens.write(context, { id, vault }, { material });
    }

    return {};
}

/** Lease an authorized connection's protocol and current credential. */
async function lease(row: Connection, tokens: TokenSource, runner: Runner): Promise<Lease> {
    // require an authorized connection
    if (row.conditions["Authorized"]?.status !== "true") {
        throw ConnectionError.of(
            "CONNECTION_NOT_AUTHORIZED",
            `connection ${row.id} is not authorized`,
        );
    }
    const credential = await tokens.current(runner, row, runner.now);

    return { protocol: row.protocol, credential };
}

/** Mark a connection revoked, leaving its credential for its controller to withdraw. */
async function revoke(
    context: MethodContextOf<typeof connection, "revoke">,
): Promise<ResultOf<typeof connection, "revoke">> {
    const target = context.target;

    return context.updateStatus({
        expiresAt: null,
        conditions: {
            ...target.conditions,
            Authorized: StatusCondition.observe(
                target.conditions["Authorized"],
                { ...REVOKED, observedGeneration: target.generation },
                context.now,
            ),
        },
    });
}

/** Revoke a connection's tokens at their server or uninstall its integration, then destroy them. */
async function withdraw(runner: Runner, withdrawn: Connection, tokens: TokenSource): Promise<void> {
    // read the credential the vault stores, none before the first was stored
    const isRetained = withdrawn.storedAt !== null;
    const token = isRetained ? (await tokens.read(runner, withdrawn)).token : undefined;

    // uninstall an installed integration through its issuer
    if (withdrawn.protocol.kind === "installation" && withdrawn.subject !== null) {
        const issued = await tokens.issuer(runner, withdrawn.integration);
        await issued.uninstall(withdrawn.subject);
    }
    // revoke the refresh token, or else the access token, at the authorization server
    else if (token !== undefined && withdrawn.integration !== null) {
        const { server } = await tokens.integration(runner, withdrawn.integration);
        await (token.refreshToken === undefined
            ? server.revoke(token.accessToken, "access_token")
            : server.revoke(token.refreshToken, "refresh_token"));
    }

    // remove the stored credential
    await tokens.remove(runner, withdrawn.vault, withdrawn.id);
}

/** Renew a connection's credential before it lapses. */
async function reconcile(
    reconciliation: ObjectReconciliation<typeof connection>,
    tokens: TokenSource,
    authorization: ConnectionAuthorization,
): Promise<number | undefined> {
    // read the key's one connection at its current revision
    const { now } = reconciliation;
    const runner = Runner.of(reconciliation.store);
    const [current] = await reconciliation.database
        .select()
        .from(connection.table)
        .where(eq(connection.table.id, nth(reconciliation.rows, 0).id));
    if (current === undefined) {
        return undefined;
    }
    const reason = Connection.authorized(current);
    const isAuthorized = current.conditions["Authorized"]?.status === "true";
    const kind = current.protocol.kind;

    // destroy a deleted connection's credential before it goes
    if (current.deletedAt !== null) {
        await finalize(reconciliation, current, tokens);

        return undefined;
    }
    // withdraw a revoked connection's credential once, then leave it
    else if (reason === "Revoked") {
        if (current.storedAt !== null) {
            await withdraw(runner, current, tokens);
            await tokens.observe(runner, current, REVOKED, { storedAt: null });
        }

        return undefined;
    }
    // mark supplied material authorized
    else if (SUPPLIED_KINDS.has(kind)) {
        if (!isAuthorized) {
            await tokens.observe(
                runner,
                current,
                { status: "true", reason: "Supplied", message: "" },
                {},
            );
        }

        return undefined;
    }
    // poll a device authorization or wait for a person to authorize the others
    else if (PERSON_KINDS.has(kind) && isAwaitingPerson(reason)) {
        return awaitPerson(reconciliation, current, tokens, authorization);
    }
    // obtain or refresh a token before it expires
    else if (reason !== "Expired") {
        return renewToken(runner, current, tokens, now);
    }
    // leave an expired grant for a person to authorize again
    else {
        return undefined;
    }
}

/** Withdraw a deleted connection's credential as a revocation does, then let it go. */
async function finalize(
    reconciliation: ObjectReconciliation<typeof connection>,
    current: Connection,
    tokens: TokenSource,
): Promise<void> {
    await withdraw(Runner.of(reconciliation.store), current, tokens);
    await reconciliation.mutate([connection.call("finalize", connection.target(current))]);
}

/** Poll a device authorization, or mark it pending for a person, returning when to look again. */
async function awaitPerson(
    reconciliation: ObjectReconciliation<typeof connection>,
    current: Connection,
    tokens: TokenSource,
    authorization: ConnectionAuthorization,
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
        return authorization.poll(reconciliation, current, request);
    }

    // mark the connection pending until a person authorizes it
    if (current.conditions["Authorized"] === undefined) {
        const message = "waiting for a person to authorize the connection";
        const pending = { status: "false", reason: "Pending", message } as const;
        await tokens.observe(Runner.of(reconciliation.store), current, pending, {});
    }

    return undefined;
}

/** Obtain a connection's token, or refresh it before it expires, returning when to look again. */
async function renewToken(
    runner: Runner,
    current: Connection,
    tokens: TokenSource,
    now: number,
): Promise<number | undefined> {
    // leave a stored token that never expires as it is
    const authorized = current.conditions["Authorized"];
    const isRetained = authorized?.status === "true" && current.storedAt !== null;
    if (isRetained && current.expiresAt === null) {
        return undefined;
    }

    // wait until its refresh is due, then obtain or refresh it
    const due = isRetained && current.expiresAt !== null ? current.expiresAt - REFRESH_MARGIN : now;
    if (due > now) {
        return due - now;
    }

    return refresh(runner, current, tokens, now);
}

/** Refresh a connection's token, recording failures besides a refused grant, then waiting. */
async function refresh(
    runner: Runner,
    row: Connection,
    tokens: TokenSource,
    now: number,
): Promise<number | undefined> {
    try {
        const { token } = await tokens.refresh(runner, row, now);
        const expiresAt = token?.expiresAt;

        return expiresAt === undefined ? undefined : Math.max(0, expiresAt - REFRESH_MARGIN - now);
    } catch (error) {
        // pass on a failure that is no connection refusal
        if (!ConnectionError.is(error)) {
            throw error;
        }
        // retry a failure besides a refused grant, keeping a token that has not lapsed leasable
        else if (!TokenSource.isGrantExpired(error)) {
            const failures = row.failures + 1;
            const isLive = row.storedAt !== null && row.expiresAt !== null && row.expiresAt > now;
            const status = isLive ? "true" : "false";
            const failing = { status, reason: "Failing", message: error.message } as const;
            await tokens.observe(runner, row, failing, { failures });

            const drawn = RetryPolicy.draw(`${row.id} ${failures}`);

            return Math.ceil(RetryPolicy.interval(RETRY, failures, () => drawn));
        }
        // leave a refused grant expired, for a person to authorize again
        else {
            return undefined;
        }
    }
}

/** Report whether a connection waits for a person: never authorized, pending, or denied. */
function isAwaitingPerson(reason: AuthorizedReason | undefined): boolean {
    return reason === undefined || reason === "Pending" || reason === "Denied";
}

/** Refuse an installation connection outside its integration's primary. */
function requireInstallable(
    kind: ConnectionKind,
    chain: readonly string[],
    tokens: TokenSource,
): void {
    if (kind === "installation" && !tokens.isPrimary(chain)) {
        throw ConnectionError.of(
            "PRIMARY_REQUIRED",
            "installation connections belong to their integration's primary",
        );
    }
}

/** Refuse a connection without the client its kind authorizes through. */
function requireClient(input: MethodContextOf<typeof connection, "create">["input"]): void {
    // read whether the kind authorizes through a client and whether the connection has one
    const { protocol, resource } = input;
    const hasClient =
        input.integration !== undefined ||
        (protocol.kind === "authorization-code" && resource !== undefined);

    // refuse a missing client
    if (!SUPPLIED_KINDS.has(protocol.kind) && !hasClient) {
        throw ConnectionError.of(
            "INTEGRATION_REQUIRED",
            protocol.kind === "authorization-code"
                ? "an authorization-code connection needs an integration, or a resource whose server registers its client"
                : `a ${protocol.kind} connection needs its integration`,
        );
    }
}

/** Refuse to authorize or revoke a revoked connection. */
function requireOpen(target: Connection): void {
    if (Connection.authorized(target) === "Revoked") {
        throw ConnectionError.of("CONNECTION_REVOKED", `connection ${target.id} is revoked`);
    }
}
