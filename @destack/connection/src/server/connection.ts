import { and, eq } from "@destack/db";
import {
    type CallOf,
    type ObjectReconciliation,
    type ResultOf,
    StatusCondition,
} from "@destack/object";
import { aligned, present } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { ConnectionError } from "../error/error.ts";
import { authorizationRequest } from "../object/authorization-request.ts";
import { Credential } from "../credential/credential.ts";
import {
    Connection,
    type ConnectionKind,
    connection,
    type Loan,
    Withdrawal,
} from "../object/connection.ts";
import { integration } from "../object/integration.ts";
import { PERSON_KINDS, REFRESH_MARGIN, SUPPLIED_KINDS, TokenSource, Runner } from "./token.ts";
import { ConnectionAuthorization } from "./authorization.ts";

/** How long a connection waits to try again after its server failed to answer: a minute. */
const RETRY_MILLISECONDS = 60_000;

/** Implement connections. */
export function implementConnection(tokens: TokenSource): typeof connection {
    return connection.implement({
        methods: {
            create: {
                authorize: async (call) => {
                    // require the home, client, vault and exchange source the connection's kind needs
                    requireHome(call.input.protocol.kind, call.chain, tokens);
                    requireClient(call.input);
                    Credential.requireVault(call.input.vault, call.scope);
                    await Connection.requireSource(call);
                },
                prepare: (call) => keep(call, tokens),
                handle: (call, next) =>
                    next(
                        call.with({
                            input: {
                                ...call.input,
                                keptAt: call.input.material === undefined ? null : call.now,
                            },
                        }),
                    ),
            },
            installations: async (call) => {
                // read the integrations with the client identifier, then the scope's authorized installations of them
                const { clientId, id, installation } = call.input;
                const integrations = await call.read(integration).findMany({ where: { clientId } });
                const ids = new Set<string>(integrations.map((row) => row.id));
                const rows = await call.read(connection).findMany({
                    where: {
                        scope: call.scope,
                        subject: installation ?? { isNotNull: true },
                        ...(id === undefined ? {} : { id }),
                    },
                });

                return {
                    items: rows.flatMap((row) =>
                        row.protocol.kind === "installation" &&
                        row.conditions["Authorized"]?.status === "true" &&
                        row.integration !== null &&
                        ids.has(row.integration.id) &&
                        row.subject !== null
                            ? [{ id: row.id, installation: row.subject }]
                            : [],
                    ),
                };
            },
            lend: {
                prepare: (call) => lend(call.requireTarget(), tokens, call),
                handle: async (call) => call.prepared,
            },
            authorize: {
                authorize: async (call) => requireOpen(call.requireTarget()),
                prepare: (call) => ConnectionAuthorization.start(call, tokens),
                handle: (call) => ConnectionAuthorization.started(call),
            },
            complete: {
                prepare: (call) => ConnectionAuthorization.exchange(call, tokens),
                handle: (call) => ConnectionAuthorization.complete(call),
            },
            revoke: {
                authorize: async (call) => requireOpen(call.requireTarget()),
                prepare: async (call) => withdrawal(call.requireTarget()),
                handle: (call) => revoke(call),
                settle: async (call, settled) => {
                    // withdraw the tokens once the revocation committed
                    if (settled.status === "committed") {
                        await withdraw(call, settled.prepared, tokens);
                    }
                },
            },
        },
        controller: {
            pending: {},
            reconcile: (reconciliation) => reconcile(reconciliation, tokens),
        },
    });
}

/** Keep the material a connection's kind sends in its vault before the connection is created. */
async function keep(call: CallOf<typeof connection, "create", unknown>, tokens: TokenSource) {
    // require material of the connection's kind for exactly the kinds that send it
    const { protocol, vault, material } = call.input;
    const isSupplied = SUPPLIED_KINDS.has(protocol.kind) || protocol.kind === "jwt";
    if (
        isSupplied !== (material !== undefined) ||
        (material !== undefined && material.kind !== protocol.kind)
    ) {
        throw new CallError("BAD_REQUEST", {
            message: isSupplied
                ? `a ${protocol.kind} connection is created with its ${protocol.kind} material`
                : `a ${protocol.kind} connection is authorized, never created with material`,
        });
    }

    // keep the material
    if (material !== undefined) {
        const id = present(call.id, "a prepared creation's identifier");
        await tokens.write(call, { id, vault }, { material });
    }

    return {};
}

/** Lend an authorized connection's protocol and current credential. */
async function lend(row: Connection, tokens: TokenSource, runner: Runner): Promise<Loan> {
    // require an authorized connection
    if (row.conditions["Authorized"]?.status !== "true") {
        throw new ConnectionError("UNAUTHORIZED", `connection ${row.id} is unauthorized`);
    }
    const credential = await tokens.current(runner, row, runner.now);

    return { protocol: row.protocol, credential };
}

/** Describe the credential a revocation withdraws, without the credential itself. */
function withdrawal(target: Connection): Withdrawal {
    return {
        id: target.id,
        protocol: target.protocol,
        vault: target.vault,
        integration: target.integration,
        subject: target.subject,
        keptAt: target.keptAt,
    };
}

/** Mark a connection revoked. */
async function revoke(
    call: CallOf<typeof connection, "revoke">,
): Promise<ResultOf<typeof connection, "revoke">> {
    const target = call.requireTarget();

    return call.updateStatus({
        expiresAt: null,
        keptAt: null,
        conditions: {
            ...target.conditions,
            Authorized: StatusCondition.observe(
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

/** Revoke a revocation's tokens at their server or uninstall its integration, then destroy it. */
async function withdraw(runner: Runner, withdrawn: Withdrawal, tokens: TokenSource): Promise<void> {
    // read the credential the vault keeps, none before the first was kept
    const isKept = withdrawn.keptAt !== null;
    const token = isKept ? (await tokens.read(runner, withdrawn)).token : undefined;

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

    // remove the kept credential
    if (isKept) {
        await tokens.remove(runner, withdrawn.vault, withdrawn.id);
    }
}

/** Keep a connection's credential current. */
async function reconcile(
    reconciliation: ObjectReconciliation<typeof connection>,
    tokens: TokenSource,
): Promise<number | undefined> {
    // read the key's one connection at its current revision
    const { now } = reconciliation;
    const runner = Runner.of(reconciliation.store);
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
    if (current.deletedAt !== null) {
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
            await tokens.observe(runner, current, supplied, {});
        }

        return undefined;
    }
    // poll a device authorization or wait for a person to authorize the others
    else if (PERSON_KINDS.has(kind) && authorized?.status !== "true") {
        return awaitPerson(reconciliation, current, tokens);
    }
    // obtain or refresh a token before it expires
    else if (authorized?.reason !== "Expired") {
        return keepToken(runner, current, tokens, now);
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
    // remove the credential, then let the connection go
    await tokens.remove(Runner.of(reconciliation.store), current.vault, current.id);
    await reconciliation.mutate([connection.call("finalize", connection.target(current))]);
}

/** Poll a device authorization, or mark it pending for a person, returning when to look again. */
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
        return ConnectionAuthorization.poll(reconciliation, current, request, tokens);
    }

    // mark the connection pending until a person authorizes it
    if (current.conditions["Authorized"] === undefined) {
        const message = "waiting for a person to authorize the connection";
        const pending = { status: "false" as const, reason: "Pending", message };
        await tokens.observe(Runner.of(reconciliation.store), current, pending, {});
    }

    return undefined;
}

/** Obtain a connection's token, or refresh it before it expires, returning when to look again. */
async function keepToken(
    runner: Runner,
    current: Connection,
    tokens: TokenSource,
    now: number,
): Promise<number | undefined> {
    // keep a kept token that never expires
    const authorized = current.conditions["Authorized"];
    const isKept = authorized?.status === "true" && current.keptAt !== null;
    if (isKept && current.expiresAt === null) {
        return undefined;
    }

    // wait until its refresh is due, then obtain or refresh it
    const due = isKept && current.expiresAt !== null ? current.expiresAt - REFRESH_MARGIN : now;
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
        // leave an expired grant for a person and retry other failures
        if (!(error instanceof ConnectionError)) {
            throw error;
        } else if (error.error !== "invalid_grant") {
            await tokens.observe(
                runner,
                row,
                { status: "false", reason: "Failing", message: error.message },
                {},
            );

            return RETRY_MILLISECONDS;
        }

        return undefined;
    }
}

/** Refuse an installation connection outside its account's home, where its integration lives. */
function requireHome(kind: ConnectionKind, chain: readonly string[], tokens: TokenSource): void {
    if (kind === "installation" && !tokens.isHome(chain)) {
        throw new CallError("BAD_REQUEST", {
            message: "installation connections live at their integration's home",
        });
    }
}

/** Refuse a connection without the client its kind authorizes through. */
function requireClient(input: CallOf<typeof connection, "create">["input"]): void {
    // read whether the kind authorizes through a client and whether the connection has one
    const { protocol, resource } = input;
    const hasClient =
        input.integration !== undefined ||
        (protocol.kind === "authorization-code" && resource !== undefined);

    // refuse a missing client
    if (!SUPPLIED_KINDS.has(protocol.kind) && !hasClient) {
        throw new CallError("BAD_REQUEST", {
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
        throw new CallError("CONFLICT", { message: `connection ${target.id} is revoked` });
    }
}
