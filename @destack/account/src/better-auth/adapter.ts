import { AccessContext, Authorization, Caller, principal } from "@destack/access";
import { RequestId, Scope } from "@destack/sync";
import { user, User } from "../object/user.ts";
import { createAdapterFactory, type CleanedWhere, type CustomAdapter } from "better-auth/adapters";
import type { DBAdapterInstance } from "better-auth";
import {
    and,
    combine,
    eq,
    isNotNull,
    isNull,
    lt,
    notExists,
    or,
    sql,
    asc,
    desc,
    type SQL,
    TABLE,
    type DatabaseConnection,
    Row,
    Transaction,
    Table,
} from "@destack/db";
import { type ObjectStore, Reservation } from "@destack/object/server";
import { Claimant } from "@destack/object";
import { present, schema } from "@destack/schema";
import { AccountHandle } from "../address/handle.ts";
import { APIError } from "better-auth/api";
import { Account, account } from "../object/account.ts";
import { space } from "../object/space.ts";
import { ResidencyCode } from "../object/residency.ts";
import {
    deviceAuthorization,
    oauthAccessToken,
    oauthClientAssertion,
    oauthRefreshToken,
    rateLimit,
    verification,
} from "../authentication/table.ts";
import { userLogin } from "../object/user-login.ts";
import { passkey } from "../object/passkey.ts";
import { session } from "../object/session.ts";
import { twoFactor } from "../object/two-factor.ts";
import { oauthClient } from "../object/oauth-client.ts";
import { oauthConsent } from "../object/oauth-consent.ts";
import {
    CLIENT_RESOURCE_MODEL,
    PackageResource,
    RESOURCE_MODEL,
    type TokenLifetime,
} from "../oauth/resource.ts";
import { CallError } from "@destack/object";
import { BetterAuthWhere } from "./where.ts";
import type { Clock } from "@destack/environment";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** The authentication tables the adapter accepts. */
const TABLES: Readonly<Record<string, Table>> = {
    user: user.table,
    userLogin: userLogin.table,
    session: session.table,
    deviceAuthorization,
    passkey: passkey.table,
    twoFactor: twoFactor.table,
    verification,
    rateLimit,
    oauthClient: oauthClient.table,
    oauthConsent: oauthConsent.table,
    oauthClientAssertion,
    oauthRefreshToken,
    oauthAccessToken,
};

/** The user Better Auth writes. */
const NewUser = schema.looseObject({
    /** The handle the personal account claims. */
    handle: AccountHandle,
    /** The jurisdiction the personal account stores data in. */
    residencyId: ResidencyCode,
});

/** The object tables with rows Better Auth creates in their user's scope. */
const USER_SCOPED_TABLES: ReadonlySet<Table> = new Set([
    userLogin.table,
    session.table,
    passkey.table,
    twoFactor.table,
    oauthConsent.table,
]);

/** The tables of credentials the provider issues to users. */
const TOKEN_TABLES: ReadonlySet<Table> = new Set([oauthAccessToken, oauthRefreshToken]);

/** Bind Better Auth's adapter protocol to the account database. */
export function betterAuthAdapter(
    database: DatabaseConnection,
    objects: Pick<ObjectStore, "mutate" | "claims" | "authorizer">,
    lifetime: TokenLifetime,
    clock: Clock,
): DBAdapterInstance {
    return (options) =>
        createAdapterFactory({
            config: {
                adapterId: "destack",
                adapterName: "Destack",
                supportsJSON: true,
                supportsBooleans: true,
                supportsDates: true,
                supportsArrays: true,
                // run at read committed
                transaction: (callback) =>
                    database.transaction(
                        (transaction) =>
                            callback(
                                betterAuthAdapter(transaction, objects, lifetime, clock)(options),
                            ),
                        { isolationLevel: "read committed" },
                    ),
                customTransformInput: ({ data, fieldAttributes }): unknown => {
                    const value: unknown = data;

                    return fieldAttributes.type === "date" && value instanceof Date
                        ? value.getTime()
                        : value;
                },
                customTransformOutput: ({ data, fieldAttributes }): unknown => {
                    const value: unknown = data;

                    return fieldAttributes.type === "date" && typeof value === "number"
                        ? new Date(value)
                        : value;
                },
            },
            adapter: () => adapter(database, objects, lifetime, clock),
        })(options);
}

/** Implement atomic authentication reads and writes over portable SQL. */
function adapter(
    database: DatabaseConnection,
    objects: Pick<ObjectStore, "mutate" | "claims" | "authorizer">,
    lifetime: TokenLifetime,
    clock: Clock,
): CustomAdapter {
    return { ...reads(database, lifetime), ...writes(database, objects, clock) };
}

/** Read, page and count authentication rows. */
function reads(
    database: DatabaseConnection,
    lifetime: TokenLifetime,
): Pick<CustomAdapter, "findOne" | "findMany" | "count"> {
    return {
        async findOne({ model, where, select }) {
            // answer a resource as the package it identifies
            if (model === RESOURCE_MODEL) {
                const resource = PackageResource.of(identifierOf(where), lifetime);

                return resource === undefined ? null : asRecord(resource);
            }

            // read the first matching row, a token only under its standing consent
            const source = table(model);
            const [row] = await database
                .select(selection(source, select))
                .from(source)
                .where(filter(database, source, where))
                .limit(1);

            return row === undefined ? null : asRecord(row);
        },
        async findMany({ model, where, select, limit, offset, sortBy }) {
            // answer the packages a resource lookup asks for, and no client-resource link
            if (model === RESOURCE_MODEL) {
                const resource = PackageResource.of(identifierOf(where), lifetime);

                return resource === undefined ? [] : [asRecord(resource)];
            } else if (model === CLIENT_RESOURCE_MODEL) {
                return [];
            }

            // select, filter, sort and page the rows, tokens only under their standing consents
            const source = table(model);
            let query = database
                .select(selection(source, select))
                .from(source)
                .where(filter(database, source, where))
                .limit(limit);

            // retain Better Auth's explicit ordering and pagination
            if (offset !== undefined) {
                query = query.offset(offset);
            }
            if (sortBy !== undefined) {
                const field = BetterAuthWhere.column(source, sortBy.field);
                query = query.orderBy(sortBy.direction === "asc" ? asc(field) : desc(field));
            }

            return (await query).map((row) => asRecord(row));
        },
        async count({ model, where }) {
            // count the matching rows, tokens only under their standing consents
            const source = table(model);
            const [row] = await database
                .select({ count: sql<number>`count(*)`.mapWith(Number) })
                .from(source)
                .where(filter(database, source, where));
            if (row === undefined) {
                throw AccountError.of("NO_COUNT", `counting ${model} returned no row`);
            }

            return row.count;
        },
    };
}

/** Filter rows by Better Auth's clauses, and tokens by their standing consents. */
function filter(
    database: DatabaseConnection,
    source: Table,
    where: CleanedWhere[] | undefined,
): SQL<boolean> | undefined {
    // combine the clauses and the standing consent that apply
    const predicates = [
        BetterAuthWhere.condition(source, where),
        standing(database, source),
    ].filter((predicate) => predicate !== undefined);

    return predicates.length === 0 ? undefined : combine(predicates, "AND");
}

/** Match the rows Better Auth may read: standing consents and the tokens they still grant. */
function standing(database: DatabaseConnection, source: Table): SQL | undefined {
    // select only the consents the user has not revoked
    const consent = oauthConsent.table;
    if (source === consent) {
        return isNull(consent.revokedAt);
    }

    // refuse a token whose consent the user revoked, or granted again after the token was issued
    const token =
        source === oauthAccessToken
            ? oauthAccessToken
            : source === oauthRefreshToken
              ? oauthRefreshToken
              : undefined;
    if (token === undefined) {
        return undefined;
    }

    return notExists(
        database
            .select({ id: consent.id })
            .from(consent)
            .where(
                and(
                    eq(consent.clientId, token.clientId),
                    eq(consent.userId, token.userId),
                    or(isNotNull(consent.revokedAt), lt(token.createdAt, consent.grantedAt)),
                ),
            ),
    );
}

/** Create, change, consume and delete authentication rows, each change one statement. */
function writes(
    database: DatabaseConnection,
    objects: Pick<ObjectStore, "mutate" | "claims" | "authorizer">,
    clock: Clock,
): Omit<CustomAdapter, "findOne" | "findMany" | "count"> {
    return {
        async create({ model, data }) {
            // insert a person with their personal account
            if (model === "user") {
                return asRecord(await createUser(database, objects, data, clock.now()));
            }
            // insert any other row
            else {
                return asRecord(await createRow(database, model, data, clock.now()));
            }
        },
        async update({ model, where, update }) {
            // update the first matching row
            const source = table(model);
            const [row] = await database
                .update(source)
                .set(revise(source, update, clock.now()))
                .where(one(database, source, where))
                .returning();

            return row === undefined ? null : asRecord(row);
        },
        async updateMany({ model, where, update }) {
            // update every matching row
            const source = table(model);
            const rows = await database
                .update(source)
                .set(revise(source, update, clock.now()))
                .where(BetterAuthWhere.condition(source, where))
                .returning({ id: BetterAuthWhere.column(source, "id") });

            return rows.length;
        },
        async delete({ model, where }) {
            // delete the first matching row
            const source = table(model);
            await database.delete(source).where(one(database, source, where));
        },
        async deleteMany({ model, where }) {
            // delete every matching row
            const source = table(model);
            const rows = await database
                .delete(source)
                .where(BetterAuthWhere.condition(source, where))
                .returning({ id: BetterAuthWhere.column(source, "id") });

            return rows.length;
        },
        async consumeOne({ model, where }) {
            // delete and return the first matching row, a token only under its standing consent
            const source = table(model);
            const [row] = await database
                .delete(source)
                .where(one(database, source, where))
                .returning();

            return row === undefined ? null : asRecord(row);
        },
        async incrementOne({ model, where, increment, set }) {
            // increment the first matching row
            const source = table(model);
            const row = await incrementRow(database, source, where, increment, set, clock.now());

            return row === undefined ? null : asRecord(row);
        },
    };
}

/** Insert a completed row, refusing a credential of a suspended or deleting user. */
async function createRow(
    database: DatabaseConnection,
    model: string,
    data: unknown,
    now: number,
): Promise<Row> {
    // refuse issuing a credential to a suspended or deleting user
    const source = table(model);
    const values = complete(source, data, now);
    if (
        TOKEN_TABLES.has(source) &&
        !(await User.maySignIn(database, schema.string().parse(values["userId"])))
    ) {
        throw new APIError("UNAUTHORIZED", { message: "user is no longer authorized" });
    }

    // grant a consent on the store's clock, granting a revoked one of the same pair again in place
    if (source === oauthConsent.table) {
        const granted = { ...values, createdAt: now, grantedAt: now, revokedAt: null };
        const [row] = await database
            .insert(source)
            .values(granted)
            .onConflictDoUpdate({
                target: [oauthConsent.table.clientId, oauthConsent.table.userId],
                set: { ...granted, id: sql`${oauthConsent.table.id}` },
                setWhere: isNotNull(oauthConsent.table.revokedAt),
            })
            .returning();

        return present(row, `the granted ${model}`);
    }

    // insert the row, a token issued on the store's clock its consents are granted on
    const issued = TOKEN_TABLES.has(source) ? { ...values, createdAt: now } : values;
    const [row] = await database.insert(source).values(issued).returning();

    return present(row, `the inserted ${model}`);
}

/** Apply the guard and counter changes to one row in a single database statement. */
async function incrementRow(
    database: DatabaseConnection,
    source: Table,
    where: CleanedWhere[],
    increment: Readonly<Record<string, number>>,
    set: Readonly<Record<string, unknown>> | undefined,
    now: number,
): Promise<Row | undefined> {
    // add each amount to its column
    const increments = Object.fromEntries(
        Object.entries(increment).map(([name, amount]) => [
            name,
            sql`${BetterAuthWhere.column(source, name)} + ${amount}`,
        ]),
    );

    // update the first matching row with the guards and increments
    const [row] = await database
        .update(source)
        .set({ ...revise(source, { ...set }, now), ...increments })
        .where(one(database, source, where))
        .returning();

    return row;
}

/** Insert a person as the owner of their scope. */
async function createUser(
    database: DatabaseConnection,
    objects: Pick<ObjectStore, "mutate" | "claims" | "authorizer">,
    data: unknown,
    now: number,
): Promise<Row> {
    // split the personal account's handle and residency from the user's columns
    const { handle, residencyId, ...person } = NewUser.parse(data);
    const source: Table = user.table;
    const values = complete(source, person, now);

    // write the user, releasing the names a lost attempt reserved
    let reservation: Reservation | undefined;
    const written = await Transaction.run(database, async (transaction) => {
        // write the user's completed row, its home its personal account's
        const userId = schema.identifier("user").parse(values["id"]);
        const personal = Account.personal(userId);
        const homed = { ...values, homeId: Account.home(personal) };
        const row = Table.complete(source, Table.values(source, homed));
        await transaction.write({ table: source, before: null, after: row });

        // make it the owner of its scope, its public profile shown to anyone
        const self = principal.user.reference(Scope.universe.id, userId);
        const caller = AccessContext.of(Caller.of(self), now);
        await new Authorization(objects.authorizer, transaction, () => caller).create(
            self,
            User.creation(self),
        );

        // create its personal account and home as the person
        await objects.mutate(
            caller,
            {
                id: RequestId.create(now),
                calls: [
                    account.call("create", {
                        ...account.target({ scope: userId, id: personal }),
                        kind: "personal",
                        handle,
                        name: handle,
                        residencyId,
                    }),
                ],
            },
            { transaction },
        );

        // reserve the handle and the home's name before the commit
        await reservation?.release();
        reservation = undefined;
        const claims = present(objects.claims, "the account objects' claim type");
        const claimants = await Claimant.written(transaction, [account, space]);
        reservation = await Reservation.reserve(claims, claimants, [account, space], {
            id: crypto.randomUUID(),
            now,
        }).catch(handleTaken);
        const reserved = reservation;
        transaction.record(async (committing) => {
            await reserved.commit(committing).catch(handleTaken);
        });

        return row;
    }).catch(async (error: unknown) => {
        // release the names once the write failed
        await reservation?.release();
        throw error;
    });

    // confirm the names once the write committed
    await reservation?.confirm();

    return written;
}

/** Report a taken name as Better Auth reports a taken handle. */
function handleTaken(error: unknown): never {
    throw CallError.of(error)?.code === "CONFLICT"
        ? new APIError("CONFLICT", { code: "HANDLE_TAKEN", message: "the handle is already taken" })
        : error;
}

/** Complete the record columns Better Auth leaves out of a created row. */
function complete(source: Table, data: unknown, now: number): Row {
    // read the written columns
    const columns = source[TABLE].columns;
    const row = readColumns(source, data);

    return {
        ...row,
        ...(columns["createdAt"] !== undefined && row["createdAt"] == null
            ? { createdAt: now }
            : {}),
        ...(columns["updatedAt"] !== undefined && row["updatedAt"] == null
            ? { updatedAt: now }
            : {}),
        ...(USER_SCOPED_TABLES.has(source) ? { scope: schema.string().parse(row["userId"]) } : {}),
    };
}

/** Advance record revisions and update times when Better Auth changes rows. */
function revise(source: Table, data: unknown, now: number) {
    // read the written columns and the revision column
    const columns = source[TABLE].columns;
    const revision = columns["revision"];
    const row = readColumns(source, data);

    return {
        ...row,
        ...(revision === undefined ? {} : { revision: sql`${revision} + 1` }),
        ...(columns["updatedAt"] !== undefined && row["updatedAt"] === undefined
            ? { updatedAt: now }
            : {}),
    };
}

/** Read the columns Better Auth writes, which its date transform leaves as JSON values. */
function readColumns(source: Table, data: unknown): Row {
    return Table.values(source, Row.parse(data));
}

/** Match at most one row, and apply the predicate in the atomic mutation. */
function one(database: DatabaseConnection, source: Table, where: CleanedWhere[]) {
    // narrow the predicate to the first matching row Better Auth may read
    const condition = filter(database, source, where);
    const id = BetterAuthWhere.column(source, "id");
    const first = database.select({ id }).from(source).where(condition).limit(1);

    return and(sql`${id} IN (${first})`, condition);
}

/** Read the resource identifier a lookup filters on, refusing any other lookup. */
function identifierOf(where: CleanedWhere[] | undefined): string {
    const [only, ...others] = where ?? [];
    if (only?.field !== "identifier" || others.length > 0 || typeof only.value !== "string") {
        throw AccountError.of(
            "LOOKUP_BY_IDENTIFIER_ONLY",
            "resources can only be looked up by identifier",
        );
    }

    return only.value;
}

/** Resolve a known authentication table. */
function table(name: string): Table {
    // refuse an unknown table
    const match = Object.hasOwn(TABLES, name) ? TABLES[name] : undefined;
    if (match === undefined) {
        throw AccountError.of(
            "UNKNOWN_AUTHENTICATION_TABLE",
            `unknown authentication table: ${name}`,
        );
    }

    return match;
}

/** Hand a row to Better Auth as the record its adapter call asks for. */
function asRecord<Result>(row: object): Result;
/**
 * Hand a row to Better Auth unchanged, its type stated by the signature above.
 *
 * NOTE #Architecture: Better Auth's adapter lets its caller choose each read's result type.
 */
function asRecord(row: object): unknown {
    return row;
}

/** Project the fields the authentication protocol requests. */
function selection(source: Table, names?: string[]) {
    return names === undefined
        ? source[TABLE].columns
        : Object.fromEntries(names.map((name) => [name, BetterAuthWhere.column(source, name)]));
}
