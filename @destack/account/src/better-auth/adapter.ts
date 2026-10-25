import { AccessContext, Authorization, Caller, principal } from "@destack/access";
import { RequestId, Scope } from "@destack/sync";
import { user, User } from "../object/user.ts";
import { createAdapterFactory, type CleanedWhere, type CustomAdapter } from "better-auth/adapters";
import type { BetterAuthPlugin, DBAdapterInstance } from "better-auth";
import {
    and,
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
import { Claimant, type ObjectType } from "@destack/object";
import { present, schema, Uuid } from "@destack/schema";
import { AccountHandle } from "../address/handle.ts";
import { APIError } from "better-auth/api";
import { Account, account } from "../object/account.ts";
import { space } from "../object/space.ts";
import { ResidencyCode } from "../object/residency.ts";
import { rateLimit, verification } from "../authentication/table.ts";
import { userLogin } from "../object/user-login.ts";
import { passkey } from "../object/passkey.ts";
import { session } from "../object/session.ts";
import { challenge } from "../object/challenge.ts";
import { factor } from "../object/factor.ts";
import { CallError } from "@destack/object";
import { BetterAuthWhere } from "./where.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** The authentication tables the adapter accepts. */
const TABLES: Readonly<Record<string, Table>> = {
    user: user.table,
    userLogin: userLogin.table,
    session: session.table,
    passkey: passkey.table,
    factor: factor.table,
    challenge: challenge.table,
    verification,
    rateLimit,
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
    factor.table,
    challenge.table,
]);

/** A Better Auth plugin model: its fields as the adapter writes them. */
type BetterAuthModel = NonNullable<BetterAuthPlugin["schema"]>[string];

/** The Better Auth field type of each object field type the account's plugin models use. */
const BETTER_AUTH_TYPES: Partial<Record<string, "string" | "number" | "boolean" | "json">> = {
    string: "string",
    enum: "string",
    state: "string",
    text: "string",
    integer: "number",
    number: "number",
    time: "number",
    boolean: "boolean",
    json: "json",
};

/** Describe an account object as the Better Auth plugin model its adapter writes. */
export function betterAuthModel(type: ObjectType): BetterAuthModel {
    // describe each column
    const fields = Object.entries(type.definition.fields).map(([name, field]) => {
        // describe a reference by its target's identifier
        const target = field.target?.();
        if (target !== undefined) {
            const references = { model: target.name, field: "id" };

            return [name, { type: "string", required: field.required, references }] as const;
        }

        // describe a value column by its type, refusing a type the adapter cannot write
        const kind = BETTER_AUTH_TYPES[field.type];
        if (kind === undefined) {
            throw new TypeError(`${type.name}.${name} has no Better Auth type for ${field.type}`);
        }

        return [name, { type: kind, required: field.required }] as const;
    });

    // add the revision every object keeps, which guarded writes compare
    const revision = { type: "number", required: false, input: false } as const;

    return { fields: { ...Object.fromEntries(fields), revision } };
}

/** Bind Better Auth's adapter protocol to the account database. */
export function betterAuthAdapter(
    database: DatabaseConnection,
    objects: Pick<ObjectStore, "mutate" | "claims" | "authorizer" | "environment">,
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
                        (transaction) => callback(betterAuthAdapter(transaction, objects)(options)),
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
            adapter: () => adapter(database, objects),
        })(options);
}

/** Implement atomic authentication reads and writes over portable SQL. */
function adapter(
    database: DatabaseConnection,
    objects: Pick<ObjectStore, "mutate" | "claims" | "authorizer" | "environment">,
): CustomAdapter {
    return { ...reads(database), ...writes(database, objects) };
}

/** Read, page and count authentication rows. */
function reads(
    database: DatabaseConnection,
): Pick<CustomAdapter, "findOne" | "findMany" | "count"> {
    return {
        async findOne({ model, where, select }) {
            // read the first matching row
            const source = table(model);
            const [row] = await database
                .select(selection(source, select))
                .from(source)
                .where(filter(source, where))
                .limit(1);

            return row === undefined ? null : asRecord(row);
        },
        async findMany({ model, where, select, limit, offset, sortBy }) {
            // select, filter, sort and page the rows
            const source = table(model);
            let query = database
                .select(selection(source, select))
                .from(source)
                .where(filter(source, where))
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
            // count the matching rows
            const source = table(model);
            const [row] = await database
                .select({ count: sql<number>`count(*)`.mapWith(Number) })
                .from(source)
                .where(filter(source, where));
            if (row === undefined) {
                throw AccountError.of("COUNT_MISSING", `counting ${model} returned no row`);
            }

            return row.count;
        },
    };
}

/** Filter rows by Better Auth's clauses. */
function filter(source: Table, where: CleanedWhere[] | undefined): SQL<boolean> | undefined {
    return BetterAuthWhere.condition(source, where);
}

/** Create, change, consume and delete authentication rows, each change one statement. */
function writes(
    database: DatabaseConnection,
    objects: Pick<ObjectStore, "mutate" | "claims" | "authorizer" | "environment">,
): Omit<CustomAdapter, "findOne" | "findMany" | "count"> {
    const { clock } = objects.environment;

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
            const [row] = await database.transaction(
                async (transaction) =>
                    await transaction
                        .update(source)
                        .set(revise(source, update, clock.now()))
                        .where(one(database, source, where))
                        .returning(),
            );

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
            await database.transaction(
                async (transaction) =>
                    await transaction.delete(source).where(one(database, source, where)),
            );
        },
        async deleteMany({ model, where }) {
            // delete every matching row
            const source = table(model);
            const rows = await database.transaction(
                async (transaction) =>
                    await transaction
                        .delete(source)
                        .where(BetterAuthWhere.condition(source, where))
                        .returning({ id: BetterAuthWhere.column(source, "id") }),
            );

            return rows.length;
        },
        async consumeOne({ model, where }) {
            // delete and return the first matching row
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

/** Insert a completed row, stamped on the store's clock. */
async function createRow(
    database: DatabaseConnection,
    model: string,
    data: unknown,
    now: number,
): Promise<Row> {
    // insert the row
    const source = table(model);
    const values = complete(source, data, now);
    const [row] = await database.transaction(
        async (transaction) => await transaction.insert(source).values(values).returning(),
    );

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
    const [row] = await database.transaction(
        async (transaction) =>
            await transaction
                .update(source)
                .set({ ...revise(source, { ...set }, now), ...increments })
                .where(one(database, source, where))
                .returning(),
    );

    return row;
}

/** Insert a person as the owner of their scope. */
async function createUser(
    database: DatabaseConnection,
    objects: Pick<ObjectStore, "mutate" | "claims" | "authorizer" | "environment">,
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
        await new Authorization(
            objects.authorizer,
            transaction,
            () => caller,
            objects.environment,
        ).create(self, User.creation(self));

        // create its personal account and home as the person
        await objects.mutate(
            caller,
            {
                id: RequestId.create(objects.environment),
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
            id: Uuid.v4(objects.environment.random),
            environment: objects.environment,
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

/** Complete a created row's record columns on the store's clock, a session's on Better Auth's. */
function complete(source: Table, data: unknown, now: number): Row {
    // read the written columns, keeping a session's times, which Better Auth's sign-in checks read on its clock
    const columns = source[TABLE].columns;
    const row = readColumns(source, data);
    const isStamped = source !== session.table;

    return {
        ...row,
        ...(columns["createdAt"] === undefined || (!isStamped && row["createdAt"] != null)
            ? {}
            : { createdAt: now }),
        ...(columns["updatedAt"] === undefined || (!isStamped && row["updatedAt"] != null)
            ? {}
            : { updatedAt: now }),
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
    // narrow the predicate to the first matching row
    const condition = filter(source, where);
    const id = BetterAuthWhere.column(source, "id");
    const first = database.select({ id }).from(source).where(condition).limit(1);

    return and(sql`${id} IN (${first})`, condition);
}

/** Resolve a known authentication table. */
function table(name: string): Table {
    // refuse an unknown table
    const match = Object.hasOwn(TABLES, name) ? TABLES[name] : undefined;
    if (match === undefined) {
        throw AccountError.of(
            "AUTHENTICATION_TABLE_UNKNOWN",
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
