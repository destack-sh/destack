import { Authorization, type Authorizer, principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { user } from "../object/user.ts";
import { createAdapterFactory, type CleanedWhere, type CustomAdapter } from "better-auth/adapters";
import type { DBAdapterInstance } from "better-auth";
import {
    and,
    or,
    sql,
    asc,
    desc,
    eq,
    ne,
    lt,
    lte,
    gt,
    gte,
    isNull,
    isNotNull,
    dialectSQL,
    TABLE,
    type Table,
    type DatabaseConnection,
} from "@destack/db";
import { deviceAuthorization } from "../stack/authentication/device.ts";
import { identity, passkey, session, twoFactor } from "../object/authentication.ts";
import { signingKey } from "../stack/authentication/key.ts";
import { verification, rateLimit } from "../stack/verification.ts";
import { oauthClient, oauthConsent } from "../object/oauth.ts";
import {
    oauthClientAssertion,
    oauthClientResource,
    oauthResource,
} from "../stack/authentication/oauth/authorization.ts";
import { oauthAccessToken, oauthRefreshToken } from "../stack/authentication/oauth/token.ts";
import { ServiceError } from "@destack/service/error";
import { authorizeSignIn } from "./session.ts";

/** Authentication tables accepted by the persistent adapter. */
const TABLES: Readonly<Record<string, Table>> = {
    user: user.table,
    identity: identity.table,
    session: session.table,
    deviceAuthorization,
    passkey: passkey.table,
    twoFactor: twoFactor.table,
    signingKey,
    verification,
    rateLimit,
    oauthClient: oauthClient.table,
    oauthConsent: oauthConsent.table,
    oauthResource,
    oauthClientResource,
    oauthClientAssertion,
    oauthRefreshToken,
    oauthAccessToken,
};

/** The object tables with rows Better Auth creates in their user's scope. */
const PERSONAL_TABLES: ReadonlySet<Table> = new Set([
    identity.table,
    session.table,
    passkey.table,
    twoFactor.table,
    oauthConsent.table,
]);

/** The tables of credentials the provider issues to users. */
const CREDENTIALS: ReadonlySet<Table> = new Set([oauthAccessToken, oauthRefreshToken]);

/** Bind Better Auth's adapter protocol to the configured Destack database. */
export function authenticationDatabase(
    database: DatabaseConnection,
    users: Authorizer,
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
                // run at read committed, as Better Auth expects
                transaction: (callback) =>
                    database.transaction(
                        (transaction) =>
                            callback(authenticationDatabase(transaction, users)(options)),
                        { isolationLevel: "read committed" },
                    ),
                customTransformInput: ({ data, fieldAttributes }) =>
                    fieldAttributes.type === "date" && data instanceof Date ? data.getTime() : data,
                customTransformOutput: ({ data, fieldAttributes }) =>
                    fieldAttributes.type === "date" && typeof data === "number"
                        ? new Date(data)
                        : data,
            },
            adapter: () => adapter(database, users),
        })(options);
}

/** Implement atomic authentication reads and writes over portable SQL. */
function adapter(database: DatabaseConnection, users: Authorizer): CustomAdapter {
    return {
        async create({ model, data }) {
            // refuse issuing a credential to a suspended or deleting user
            const source = table(model);
            if (CREDENTIALS.has(source)) {
                await authorizeSignIn(data.userId as string, database);
            }

            // insert a user with the scope it owns, in one transaction
            if (source === (user.table as Table)) {
                return database.transaction(async (transaction) => {
                    // insert the user, then make it the owner of its scope
                    const [row] = await transaction
                        .insert(source)
                        .values(complete(source, data))
                        .returning();
                    const self = principal.user.reference(Scope.universe.id, String(row!.id));
                    const caller = { subjects: [self], now: Date.now(), attributes: {} };
                    await new Authorization(users, transaction, () => caller).create(self, {
                        owner: self,
                    });

                    return row as typeof data;
                });
            }

            // insert any other completed row
            const [row] = await database.insert(source).values(complete(source, data)).returning();

            return row as typeof data;
        },
        async findOne<Result>({ model, where, select }: Parameters<CustomAdapter["findOne"]>[0]) {
            const source = table(model);
            const [row] = await database
                .select(selection(source, select))
                .from(source)
                .where(predicate(source, where))
                .limit(1);

            return (row ?? null) as Result | null;
        },
        async findMany<Result>({
            model,
            where,
            select,
            limit,
            offset,
            sortBy,
        }: Parameters<CustomAdapter["findMany"]>[0]) {
            // select, filter, sort and page the rows
            const source = table(model);
            const query = database
                .select(selection(source, select))
                .from(source)
                .where(predicate(source, where))
                .limit(limit);

            // retain Better Auth's explicit ordering and pagination
            if (offset !== undefined) {
                query.offset(offset);
            }
            if (sortBy) {
                const field = column(source, sortBy.field);
                query.orderBy(sortBy.direction === "asc" ? asc(field) : desc(field));
            }

            return (await query) as Result[];
        },
        async count({ model, where }) {
            const source = table(model);
            const [row] = await database
                .select({ count: sql<number>`count(*)`.mapWith(Number) })
                .from(source)
                .where(predicate(source, where));

            return row.count;
        },
        async update<Result>({
            model,
            where,
            update,
        }: {
            model: string;
            where: CleanedWhere[];
            update: Result;
        }) {
            const source = table(model);
            const [row] = await database
                .update(source)
                .set(revise(source, update as Record<string, unknown>))
                .where(one(database, source, where))
                .returning();

            return (row ?? null) as Result | null;
        },
        async updateMany({ model, where, update }) {
            const source = table(model);
            const rows = await database
                .update(source)
                .set(revise(source, update))
                .where(predicate(source, where))
                .returning({ id: column(source, "id") });

            return rows.length;
        },
        async delete({ model, where }) {
            const source = table(model);
            await database.delete(source).where(one(database, source, where));
        },
        async deleteMany({ model, where }) {
            const source = table(model);
            const rows = await database
                .delete(source)
                .where(predicate(source, where))
                .returning({ id: column(source, "id") });

            return rows.length;
        },
        async consumeOne<Result>({
            model,
            where,
        }: Parameters<NonNullable<CustomAdapter["consumeOne"]>>[0]) {
            const source = table(model);
            const [row] = await database
                .delete(source)
                .where(one(database, source, where))
                .returning();

            return (row ?? null) as Result | null;
        },
        async incrementOne<Result>({
            model,
            where,
            increment,
            set,
        }: Parameters<NonNullable<CustomAdapter["incrementOne"]>>[0]) {
            // apply the guard and counter changes in a single database statement
            const source = table(model);
            const changes: Record<string, unknown> = { ...set };
            for (const [name, amount] of Object.entries(increment)) {
                changes[name] = sql`${column(source, name)} + ${amount}`;
            }
            const [row] = await database
                .update(source)
                .set(revise(source, changes))
                .where(one(database, source, where))
                .returning();

            return (row ?? null) as Result | null;
        },
    };
}

/** Complete the record columns Better Auth leaves out. */
function complete(source: Table, data: Record<string, unknown>): Record<string, unknown> {
    const columns = source[TABLE].columns;
    const now = Date.now();

    return {
        ...data,
        ...(columns.createdAt && data.createdAt == null ? { createdAt: now } : {}),
        ...(columns.updatedAt && data.updatedAt == null ? { updatedAt: now } : {}),
        ...(PERSONAL_TABLES.has(source) ? { scope: data.userId } : {}),
    };
}

/** Advance record revisions and update times when Better Auth changes rows. */
function revise(source: Table, values: Record<string, unknown>): Record<string, unknown> {
    const columns = source[TABLE].columns;

    return {
        ...values,
        ...(columns.revision ? { revision: sql`${columns.revision} + 1` } : {}),
        ...(columns.updatedAt && values.updatedAt === undefined ? { updatedAt: Date.now() } : {}),
    };
}

/** Select at most one row while retaining the predicate on the atomic mutation. */
function one(database: DatabaseConnection, source: Table, where: CleanedWhere[]) {
    // narrow the predicate to the first matching row
    const condition = predicate(source, where);
    const id = column(source, "id");
    const selected = database.select({ id }).from(source).where(condition).limit(1);

    return and(condition, sql`${id} IN (${selected})`);
}

/** Resolve only the explicitly supported authentication tables. */
function table(name: string): Table {
    if (!Object.hasOwn(TABLES, name)) {
        throw new ServiceError("PRECONDITION_FAILED", {
            message: `unknown authentication table: ${name}`,
        });
    }

    return TABLES[name];
}

/** Resolve a declared authentication column. */
function column(source: Table, name: string) {
    const columns = source[TABLE].columns;
    if (!Object.hasOwn(columns, name)) {
        throw new ServiceError("PRECONDITION_FAILED", {
            message: `unknown authentication column: ${name}`,
        });
    }

    return columns[name];
}

/** Project only fields requested by the authentication protocol. */
function selection(source: Table, names?: string[]) {
    return names === undefined
        ? source[TABLE].columns
        : Object.fromEntries(names.map((name) => [name, column(source, name)]));
}

/** Combine Better Auth's conjunction and disjunction groups. */
function predicate(source: Table, where: CleanedWhere[] = []) {
    const conjunction = where
        .filter((entry) => entry.connector !== "OR")
        .map((entry) => comparison(source, entry));
    const disjunction = where
        .filter((entry) => entry.connector === "OR")
        .map((entry) => comparison(source, entry));

    return and(and(...conjunction), or(...disjunction));
}

/** Bind comparison values without accepting executable SQL from authentication inputs. */
function comparison(source: Table, where: CleanedWhere) {
    // match strings case insensitively when requested
    const field = column(source, where.field);
    const insensitive =
        where.mode === "insensitive" &&
        (typeof where.value === "string" ||
            (Array.isArray(where.value) &&
                where.value.every((value) => typeof value === "string")));
    const left = insensitive ? sql`lower(${field})` : field;
    const value =
        insensitive && typeof where.value === "string" ? where.value.toLowerCase() : where.value;

    // retain null and set comparisons as SQL operations
    switch (where.operator) {
        case "eq":
            return value === null ? isNull(left) : eq(left, value);
        case "ne":
            return value === null ? isNotNull(left) : ne(left, value);
        case "lt":
            return lt(left, value);
        case "lte":
            return lte(left, value);
        case "gt":
            return gt(left, value);
        case "gte":
            return gte(left, value);
        case "in":
        case "not_in": {
            const values = (value as (string | number)[]).map(
                (item) =>
                    sql`${insensitive && typeof item === "string" ? item.toLowerCase() : item}`,
            );
            const member =
                values.length === 0 ? sql`false` : sql`${left} IN (${sql.join(values, sql`, `)})`;

            return where.operator === "in" ? member : sql`NOT (${member})`;
        }
        case "contains":
            return dialectSQL({
                sqlite: sql`instr(${left}, ${value}) > 0`,
                postgresql: sql`strpos(${left}, ${value}) > 0`,
            });
        case "starts_with":
            return sql`substr(${left}, 1, length(${value})) = ${value}`;
        case "ends_with":
            return sql`substr(${left}, length(${left}) - length(${value}) + 1) = ${value}`;
    }
}
