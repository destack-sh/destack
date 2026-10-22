import { copyFile, mkdir, mkdtemp, rename, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import postgres from "postgres";
import { v7 } from "uuid";
import type { DatabaseConnection } from "../database/connection.ts";
import type { Model } from "../query/model.ts";
import { Dialect } from "../dialect/dialect.ts";
import type { Table } from "../table/table.ts";
import type * as declaration from "../declare/database.ts";
import { declareState } from "../migration/state.ts";
import * as sqlite from "../bun/connection.ts";
import * as postgresql from "../postgres/connection.ts";
import { PostgresDatabase } from "../postgres/database.ts";
import { ColumnCipher } from "../database/cipher.ts";
import { LOG_EPOCH, LOG_HORIZON } from "../log/schema.ts";
import { createLog } from "../log/trigger.ts";
import { sql } from "../sql/index.ts";
import type { Channel } from "../channel/channel.ts";
import { channelHub } from "../channel/channel.ts";
import { STATE, type TableState } from "../migration/state.ts";
import { upsertPredefined } from "../migration/plan.ts";
import { inject, type TestProject, testTarget } from "@destack/test";
import type { JsonValue } from "@destack/schema";
import type { TestTemplate, TemplateImage, ImageIndex } from "./template.ts";

/** The cipher of every test database, a fixed key so schemas reused across test processes open. */
export const TEST_CIPHER = new ColumnCipher(new Uint8Array(32).fill(7));

/** The connections each test database pools: four multi-database test workers stay under 100. */
const TEST_POOL_CONNECTIONS = 2;

/** The seconds a test pool leaves an idle connection open. */
const TEST_IDLE_SECONDS = 1;

/** How long an unclaimed test schema stays for reuse, in milliseconds: an hour of runs. */
const SCHEMA_RETENTION_MILLISECONDS = 60 * 60 * 1000;

/** The run's provided values the test databases read. */
declare module "vitest" {
    /** The values a run's setup provides to its tests. */
    export interface ProvidedContext {
        /** The images of the run's templates by name and dialect, absent without a setup. */
        templates?: ImageIndex;
    }
}

/** An isolated database for one test: a SQLite memory or file database, or a PostgreSQL schema. */
export class TestDatabase<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> {
    /** The first connection. */
    readonly database: TestConnection<Models>;
    /** Open another connection, absent for SQLite in memory. */
    readonly #open: TestConnector | undefined;
    /** Remove the database. */
    readonly #remove: () => Promise<void>;
    /** The other open connections. */
    readonly #connections = new Set<TestConnection>();

    /** Create the test database. */
    private constructor(
        database: TestConnection<Models>,
        open: TestConnector | undefined,
        remove: () => Promise<void>,
    ) {
        this.database = database;
        this.#open = open;
        this.#remove = remove;
    }

    /** Wait until a database meets a check after a commit, failing once some milliseconds pass. */
    static async settle(
        database: DatabaseConnection,
        check: () => Promise<boolean>,
        milliseconds: number,
    ): Promise<void> {
        if (!(await database.log.until(check, AbortSignal.timeout(milliseconds)))) {
            throw new TypeError(`the database never settled within ${milliseconds} ms`);
        }
    }

    /** Read the PostgreSQL server address the tests run against, refusing a run without one. */
    static address(): string {
        const address = process.env["DESTACK_TEST_POSTGRES"];
        if (address === undefined || address === "") {
            throw new TypeError("DESTACK_TEST_POSTGRES has no PostgreSQL server address");
        }

        return address;
    }

    /** Read the database the test's project runs against, or a given project's in its setup. */
    static target(project?: Pick<TestProject, "getProvidedContext">): Dialect {
        return Dialect.parse(testTarget("database", project));
    }

    /** Create an isolated database, empty or migrated, in the project's database unless given. */
    static async create<
        Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
    >(
        tables: declaration.Database<Models> | readonly Table[],
        options: TestDatabaseOptions = {},
    ): Promise<TestDatabase<Models>> {
        // read the tables and the database, the project's unless given
        const declared = "tables" in tables ? tables.tables : tables;
        const dialect = options.dialect ?? TestDatabase.target();

        // claim or create a PostgreSQL schema, or open SQLite in memory or as a file copy
        if (dialect === "postgresql") {
            return TestDatabase.#postgres(declared, tables, options);
        } else if (options.isMigrated !== true && (options.storage ?? "memory") === "memory") {
            return new TestDatabase(
                keyed(await sqlite.connectBunSqlite(":memory:", tables)),
                undefined,
                async () => {},
            );
        }

        return TestDatabase.#sqliteFile(declared, tables, options);
    }

    /** Copy a template's image in the run or in given images, with its write's value. */
    static async copy<Models extends Readonly<Record<string, Model>>, Value extends JsonValue>(
        template: TestTemplate<Value, Models>,
        run: {
            /** The images of the run's templates, the test run's by default. */
            readonly images?: ImageIndex | undefined;
            /** The database the images are of, the test project's by default. */
            readonly dialect?: Dialect;
        } = {},
    ): Promise<{ readonly storage: TestDatabase<Models>; readonly value: Value }> {
        // read the template's image in the run, refusing a template no setup of the run lists
        const images = run.images ?? inject("templates");
        const dialect = run.dialect ?? TestDatabase.target();
        const image = images?.[template.name]?.[dialect];
        if (image === undefined) {
            throw new TypeError(
                `the run has no ${dialect} image of template ${template.name}: list it in its package's test templates`,
            );
        }

        // copy it into a claimed schema or a temporary file
        const { tables } = template;
        const declared = "tables" in tables ? tables.tables : tables;
        const storage =
            image.source.dialect === "postgresql"
                ? await TestDatabase.#postgres(declared, tables, { isMigrated: true }, image.source)
                : await TestDatabase.#sqliteFile(declared, tables, {}, image.source.file);

        return { storage, value: template.value.parse(image.value) };
    }

    /** Write a template's image once for a run: a file or a schema the run stores. */
    static async image<Value extends JsonValue, Models extends Readonly<Record<string, Model>>>(
        template: TestTemplate<Value, Models>,
        dialect: Dialect,
        run: { readonly id: string; readonly directory: string },
    ): Promise<TemplateImage> {
        // name the template's database by the digest of its name
        const { tables } = template;
        const declared = "tables" in tables ? tables.tables : tables;
        const name = (await digestOf(template.name)).slice(0, 16);

        // write it into a migrated schema that stays until the run's end
        if (dialect === "postgresql") {
            const address = TestDatabase.address();
            const schema = `test_t${run.id}_${name}`;
            const session = administration(address);
            await session.unsafe(`CREATE SCHEMA "${schema}"`);
            const open: TestConnector = (connected) =>
                postgresql.connectPostgres(schemaPool(address, schema), connected).then(keyed);
            const migrated = await open(tables);
            await migrated.apply(
                await migrated.plan({
                    declared: declareState(declared, "postgresql", {
                        isReplica: false,
                        replicated: replicatedOf(tables),
                    }),
                }),
            );
            await migrated.close();
            const value = await TestDatabase.#write(
                template,
                new TestDatabase(await open(tables), open, async () => {}),
                run.directory,
            );
            const columns = await readColumns(session, schema);
            await session.end();

            return { source: { dialect, schema, columns }, value };
        }

        // write a migrated file in the run's directory, and checkpoint its WAL into it
        const file = join(run.directory, `${name}.db`);
        await copyFile(await sqliteTemplate(declared, false, replicatedOf(tables)), file);
        const value = await TestDatabase.#write(
            template,
            await TestDatabase.#sqliteAt(file, tables, async () => {}),
            run.directory,
        );
        const database = await sqlite.connectBunSqlite(file, tables);
        await database.executeScript("PRAGMA wal_checkpoint(TRUNCATE)");
        await database.close();

        return { source: { dialect, file }, value };
    }

    /** Store SQLite in the caller's file, migrated when the file is new. */
    static async at<Models extends Readonly<Record<string, Model>>>(
        file: string,
        tables: declaration.Database<Models> | readonly Table[],
    ): Promise<TestDatabase<Models>> {
        // copy the migrated database into a new file
        const declared = "tables" in tables ? tables.tables : tables;
        if (!(await Bun.file(file).exists())) {
            await copyFile(await sqliteTemplate(declared, false, replicatedOf(tables)), file);
        }

        return TestDatabase.#sqliteAt(file, tables, async () => {});
    }

    /** Drop the expired schemas no test claims and the images of runs that ended long ago. */
    static sweep(): Promise<void> {
        return TestSchema.sweep(TestDatabase.address());
    }

    /** Write a template into a database and a directory of the run's, then close the database. */
    static async #write<Value extends JsonValue, Models extends Readonly<Record<string, Model>>>(
        template: TestTemplate<Value, Models>,
        storage: TestDatabase<Models>,
        directory: string,
    ): Promise<Value> {
        const files = await mkdtemp(join(directory, "files-"));
        try {
            return await template.write(storage, files);
        } finally {
            await storage.close();
        }
    }

    /** Claim a migrated PostgreSQL schema reset to a template's rows, or create an empty one. */
    static async #postgres<Models extends Readonly<Record<string, Model>>>(
        declared: readonly Table[],
        tables: declaration.Database<Models> | readonly Table[],
        options: TestDatabaseOptions,
        template?: PostgresImage,
    ): Promise<TestDatabase<Models>> {
        // claim a migrated schema with the template's rows, locked by the claim's session
        const address = TestDatabase.address();
        if (options.isMigrated === true && declared.length > 0) {
            const state = declareState(declared, "postgresql", {
                isReplica: options.isReplica ?? false,
                replicated: replicatedOf(tables),
            });
            const schema = await TestSchema.claim(address, state, tables, template);
            const open: TestConnector = (connected) => schema.connect(connected);

            return new TestDatabase(await open(tables), open, () => schema.release());
        }

        // create an empty schema for no tables or no migration, dropped once the database closes
        const session = administration(address);
        const schema = `test_${crypto.randomUUID().replaceAll("-", "")}`;
        await session.unsafe(`CREATE SCHEMA "${schema}"`);
        const open: TestConnector = (connected) =>
            postgresql.connectPostgres(schemaPool(address, schema), connected).then(keyed);

        return new TestDatabase(await open(tables), open, async () => {
            await session.unsafe(`DROP SCHEMA "${schema}" CASCADE`);
            await session.end();
        });
    }

    /** Store SQLite in a temporary file, copied from a template's or the migrated database. */
    static async #sqliteFile<Models extends Readonly<Record<string, Model>>>(
        declared: readonly Table[],
        tables: declaration.Database<Models> | readonly Table[],
        options: TestDatabaseOptions,
        template?: string,
    ): Promise<TestDatabase<Models>> {
        // copy the template's or the migrated database into a temporary file
        const directory = await mkdtemp(join(tmpdir(), "destack-test-"));
        const file = join(directory, "test.db");
        const source =
            template ??
            (options.isMigrated === true
                ? await sqliteTemplate(declared, options.isReplica ?? false, replicatedOf(tables))
                : undefined);
        if (source !== undefined) {
            await copyFile(source, file);
        }

        return TestDatabase.#sqliteAt(file, tables, () => rm(directory, { recursive: true }));
    }

    /** Store SQLite in a file, sharing one hub, removed as given once closed. */
    static async #sqliteAt<Models extends Readonly<Record<string, Model>>>(
        file: string,
        tables: declaration.Database<Models> | readonly Table[],
        remove: () => Promise<void>,
    ): Promise<TestDatabase<Models>> {
        // share each named channel through one hub
        const hubs = new Map<string, () => Channel<unknown>>();
        const openChannel = (name: string) => {
            const party = hubs.get(name) ?? channelHub<unknown>();
            hubs.set(name, party);

            return party();
        };
        const open: TestConnector = async (connected) => {
            // commit without syncing the file, which a test database has no use for
            const opened = await sqlite.connectBunSqlite(file, connected, { openChannel });
            await opened.executeScript("PRAGMA synchronous = OFF");

            return keyed(opened);
        };

        // start a new epoch for a logged copy, as a restore does
        const database = await open(tables);
        const [log] = await database.all(
            sql`SELECT name FROM sqlite_master WHERE name = ${LOG_EPOCH}`,
        );
        if (log !== undefined) {
            await database.log.renew();
        }

        return new TestDatabase(database, open, remove);
    }

    /** Open another connection to the same database. */
    async connect<Other extends Readonly<Record<string, Model>>>(
        tables: declaration.Database<Other> | readonly Table[],
    ): Promise<TestConnection<Other>> {
        // require a reachable database
        if (this.#open === undefined) {
            throw new TypeError("an in-memory SQLite database has no further connections");
        }

        // track the connection, which closes once however often it is closed
        const connection = await this.#open(tables);
        this.#connections.add(connection);

        return connection;
    }

    /** Close every connection and remove the database. */
    async close(): Promise<void> {
        await Promise.all([...this.#connections].map((connection) => connection.close()));
        await this.database.close();
        await this.#remove();
    }
}

/** The options of a test database. */
export interface TestDatabaseOptions {
    /** The database, the project's by default, given only by tests of one database's behavior. */
    readonly dialect?: Dialect;
    /** Where empty SQLite lives. */
    readonly storage?: "memory" | "file";
    /** Whether the database starts migrated to its tables. */
    readonly isMigrated?: boolean;
    /** Whether it migrates as a replica, without trees and references. */
    readonly isReplica?: boolean;
}

/** A template image's PostgreSQL schema, whose rows claimed schemas copy. */
type PostgresImage = Extract<TemplateImage["source"], { readonly dialect: "postgresql" }>;

/** A migrated schema one test database claims, its session with the claim's lock. */
class TestSchema {
    /** The server address. */
    readonly address: string;
    /** The session with the claim's advisory lock until released. */
    readonly session: postgres.Sql;
    /** The schema's name. */
    readonly name: string;
    /** The relations and applied state the schema was claimed with. */
    readonly shape: TestSchemaShape;

    /** Store a claimed schema with its session and shape. */
    private constructor(
        address: string,
        session: postgres.Sql,
        name: string,
        shape: TestSchemaShape,
    ) {
        // store the claim's session, schema and shape
        this.address = address;
        this.session = session;
        this.name = name;
        this.shape = shape;
    }

    /** Claim a migrated schema reset to a template's rows: a free one, else a new one. */
    static async claim(
        address: string,
        state: readonly TableState[],
        tables: declaration.Database | readonly Table[],
        template?: PostgresImage,
    ): Promise<TestSchema> {
        // derive the schema names from the digest of the state and the log it migrates beside
        const session = administration(address);
        const layout = JSON.stringify([state, createLog("postgresql", "")]);
        const prefix = `test_s${(await digestOf(layout)).slice(0, 16)}_`;

        // claim a free schema of the same state whose shape still matches its stamp
        const listed = await session<{ name: string }[]>`
            SELECT nspname AS name FROM pg_namespace WHERE starts_with(nspname, ${prefix})`;
        for (const { name } of listed) {
            if (await lockListed(session, name)) {
                // read the shape of a schema whose migration finished, which its stamp marks last
                const stamp = await readStamp(session, name);
                const shape = stamp === undefined ? undefined : await readShape(session, name);
                if (shape !== undefined && stamp === (await digestOf(shape.fingerprint))) {
                    const schema = new TestSchema(address, session, name, shape);
                    await schema.#stamp();
                    await schema.#reset(state, template);

                    return schema;
                }
                await drop(session, name);
            }
        }

        // create and migrate a new schema with its claim time
        const name = `${prefix}${crypto.randomUUID().replaceAll("-", "")}`;
        await lock(session, name);
        await session.unsafe(`CREATE SCHEMA "${name}"`);
        const database = keyed(new PostgresDatabase(schemaPool(address, name), tables));
        await database.apply(await database.plan({ declared: state }));
        await database.close();
        const schema = new TestSchema(address, session, name, await readShape(session, name));
        await schema.#stamp();
        if (template !== undefined) {
            await schema.#reset(state, template);
        }

        return schema;
    }

    /** Drop the expired schemas no test claims and the templates of runs that ended long ago. */
    static async sweep(address: string): Promise<void> {
        // list the claimed schemas and run templates past retention
        const session = administration(address);
        const expired = Date.now() - SCHEMA_RETENTION_MILLISECONDS;
        const schemas = await session<{ name: string }[]>`
            SELECT nspname AS name FROM pg_namespace
            WHERE (starts_with(nspname, 'test_s') OR starts_with(nspname, 'test_t'))
                AND split_part(coalesce(obj_description(oid, 'pg_namespace'), '0'), ' ', 1)::bigint < ${expired}`;

        // drop each one no session has claimed
        for (const { name } of schemas) {
            if (await lockListed(session, name)) {
                await drop(session, name);
            }
        }
        await session.end();
    }

    /** Connect to the schema over a separate pool. */
    async connect<Models extends Readonly<Record<string, Model>>>(
        tables: declaration.Database<Models> | readonly Table[],
    ): Promise<TestConnection<Models>> {
        return keyed(new PostgresDatabase(schemaPool(this.address, this.name), tables));
    }

    /** Release the claim, leaving the schema for the next test of its state. */
    async release(): Promise<void> {
        await this.session.end();
    }

    /** Stamp the schema with its claim time and its shape's digest. */
    async #stamp(): Promise<void> {
        const digest = await digestOf(this.shape.fingerprint);
        await this.session.unsafe(`COMMENT ON SCHEMA "${this.name}" IS '${Date.now()} ${digest}'`);
    }

    /** Replace the rows with a template's and start a new epoch, in one statement batch. */
    async #reset(state: readonly TableState[], template?: PostgresImage): Promise<void> {
        // pick the tables to empty and the sequences to restart
        const quoted = (name: string) => `"${this.name}"."${name}"`;
        const emptied = this.shape.relations.filter(
            (relation) =>
                relation.kind !== "S" &&
                relation.name !== STATE &&
                relation.name !== LOG_EPOCH &&
                relation.name !== LOG_HORIZON,
        );
        const sequences = this.shape.relations.filter((relation) => relation.kind === "S");

        // empty the tables, restart sequences and horizon, then copy template or predefined rows
        const copied =
            template === undefined
                ? state.flatMap((table) =>
                      upsertPredefined(table).map((statement) => `${statement};`),
                  )
                : this.#copy(template);

        // run the batch as one implicit transaction without triggers, in a new epoch
        await this.session.unsafe(`SET LOCAL session_replication_role = replica;
            SET LOCAL search_path = "${this.name}";
            ${emptied.map((table) => `DELETE FROM ${quoted(table.name)};`).join("\n")}
            ${sequences.map((sequence) => `ALTER SEQUENCE ${quoted(sequence.name)} RESTART;`).join("\n")}
            UPDATE ${quoted(LOG_HORIZON)} SET sequence = 0 WHERE slot = 1;
            ${copied.join("\n")}
            UPDATE ${quoted(LOG_EPOCH)} SET epoch = '${v7()}' WHERE slot = 1;`);
    }

    /** Write the statements copying a template's rows, sequences and horizon into the schema. */
    #copy(template: PostgresImage): string[] {
        // copy the stored columns of each table but the state and the epoch
        const quoted = (name: string) => `"${this.name}"."${name}"`;
        const source = (name: string) => `"${template.schema}"."${name}"`;
        const rows = Object.entries(template.columns)
            .filter(([table]) => table !== STATE && table !== LOG_EPOCH && table !== LOG_HORIZON)
            .map(
                ([table, columns]) =>
                    `INSERT INTO ${quoted(table)} (${columns}) OVERRIDING SYSTEM VALUE SELECT ${columns} FROM ${source(table)};`,
            );

        // copy the sequences' positions and the horizon
        const sequences = this.shape.relations
            .filter((relation) => relation.kind === "S")
            .map(
                (sequence) =>
                    `SELECT setval('${quoted(sequence.name)}', last_value, is_called) FROM ${source(sequence.name)};`,
            );
        const horizon = `UPDATE ${quoted(LOG_HORIZON)} SET sequence = (SELECT sequence FROM ${source(LOG_HORIZON)} WHERE slot = 1) WHERE slot = 1;`;

        return [...rows, ...sequences, horizon];
    }
}

/** Open a session that manages schemas and takes claims, skipping the array type read. */
function administration(address: string): postgres.Sql {
    return postgres(address, { max: 1, fetch_types: false, onnotice: postgresql.reportNotice });
}

/** Open a connection pool whose queries resolve in a schema. */
function schemaPool(address: string, schema: string): postgres.Sql {
    return postgres(address, {
        max: TEST_POOL_CONNECTIONS,
        idle_timeout: TEST_IDLE_SECONDS,
        // commit without waiting for the WAL flush, which a test schema has no use for
        connection: { search_path: schema, synchronous_commit: "off" },
        onnotice: postgresql.reportNotice,
    });
}

/** Digest a text as hexadecimal SHA-256. */
async function digestOf(text: string): Promise<string> {
    const bytes = new TextEncoder().encode(text);

    return new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)).toHex();
}

/** Take a schema's advisory lock in a session, reporting whether it was free. */
async function lock(session: postgres.Sql, name: string): Promise<boolean> {
    const isLocked: unknown = (
        await session`SELECT pg_try_advisory_lock(hashtext(${name}))`.values()
    )[0]?.[0];
    if (typeof isLocked !== "boolean") {
        throw new TypeError("the advisory lock returned no boolean");
    }

    return isLocked;
}

/** Take a listed schema's advisory lock, reporting whether it was free and still exists. */
async function lockListed(session: postgres.Sql, name: string): Promise<boolean> {
    if (!(await lock(session, name))) {
        return false;
    }

    // release a name another session dropped after the listing
    const isPresent: unknown = (
        await session`SELECT to_regnamespace(${name}) IS NOT NULL`.values()
    )[0]?.[0];
    if (typeof isPresent !== "boolean") {
        throw new TypeError("the schema lookup returned no boolean");
    } else if (!isPresent) {
        await session`SELECT pg_advisory_unlock(hashtext(${name}))`;
    }

    return isPresent;
}

/** Read a schema's shape digest once its migration finished, absent before. */
async function readStamp(session: postgres.Sql, name: string): Promise<string | undefined> {
    const comment: unknown = (
        await session`SELECT obj_description(to_regnamespace(${name}), 'pg_namespace')`.values()
    )[0]?.[0];

    return typeof comment === "string" ? comment.split(" ")[1] : undefined;
}

/** Drop a locked schema, then release its lock. */
async function drop(session: postgres.Sql, name: string): Promise<void> {
    await session.unsafe(`DROP SCHEMA "${name}" CASCADE`);
    await session`SELECT pg_advisory_unlock(hashtext(${name}))`;
}

/** Read the stored columns of a schema's tables, quoted and listed in declared order. */
async function readColumns(
    session: postgres.Sql,
    schema: string,
): Promise<Readonly<Record<string, string>>> {
    const rows = await session<{ table: string; columns: string }[]>`
        SELECT table_name AS table, string_agg(quote_ident(column_name::text), ', ' ORDER BY ordinal_position) AS columns
        FROM information_schema.columns
        WHERE table_schema = ${schema} AND is_generated = 'NEVER'
        GROUP BY table_name`;

    return Object.fromEntries(rows.map((row) => [row.table, row.columns]));
}

/** Read a schema's tables, sequences and applied state for the migration plan to compare. */
async function readShape(session: postgres.Sql, name: string): Promise<TestSchemaShape> {
    // read the relations through the namespace dependency index
    const relations = await session<TestRelation[]>`
        SELECT c.relname AS name, c.relkind AS kind
        FROM pg_depend d JOIN pg_class c ON c.oid = d.objid
        WHERE d.classid = 'pg_class'::regclass AND d.refclassid = 'pg_namespace'::regclass
            AND d.refobjid = ${name}::regnamespace AND c.relkind IN ('r', 'p', 'S')
        ORDER BY c.relname`;

    // digest the applied state
    const digest: unknown = (
        await session`
        SELECT md5(string_agg("table" || ':' || state, ',' ORDER BY "table"))
        FROM ${session(name)}.${session(STATE)}`.values()
    )[0]?.[0];
    if (typeof digest !== "string" && digest !== null) {
        throw new TypeError("the applied state digest is not text");
    }

    return { relations, fingerprint: JSON.stringify([relations, digest]) };
}

/** The relations and applied state of a migrated test schema. */
interface TestSchemaShape {
    /** The tables and sequences by name. */
    readonly relations: readonly TestRelation[];
    /** The relations and applied state as one comparable text. */
    readonly fingerprint: string;
}

/** A table or sequence of a test schema. */
interface TestRelation {
    /** The relation name. */
    readonly name: string;
    /** The relation kind: `r` for a table, `p` for a partitioned table, `S` for a sequence. */
    readonly kind: "r" | "p" | "S";
}

/** Read the SQL names of the tables a declared database replicates, none for bare tables. */
function replicatedOf(tables: declaration.Database | readonly Table[]): ReadonlySet<string> {
    return "tables" in tables ? tables.replicated : new Set<string>();
}

/** Migrate a SQLite file once per declared state, stored on disk under its digest. */
async function sqliteTemplate(
    tables: readonly Table[],
    isReplica: boolean,
    replicated: ReadonlySet<string>,
): Promise<string> {
    // name the file by the digest of the state and the log it migrates beside
    const key = JSON.stringify(declareState(tables, "sqlite", { isReplica, replicated }));
    const layout = JSON.stringify([key, createLog("sqlite", "")]);
    const directory = join(tmpdir(), "destack-template");
    const file = join(directory, `${await digestOf(layout)}.db`);

    // reuse the file another test migrated for the state and log
    if (await Bun.file(file).exists()) {
        return file;
    }

    // migrate a file beside it and checkpoint its WAL, then move it in whole
    await mkdir(directory, { recursive: true });
    const written = join(directory, `${v7()}.db`);
    const database = await sqlite.connectBunSqlite(written, tables);
    await database.migrate(tables, { isReplica, replicated });
    await database.executeScript("PRAGMA wal_checkpoint(TRUNCATE)");
    await database.close();
    await rename(written, file);

    return file;
}

/** A connection to a test database. */
type TestConnection<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> = DatabaseConnection<Models> & { readonly close: () => Promise<void> };

/** Open a connection to a test database. */
type TestConnector = <Models extends Readonly<Record<string, Model>>>(
    tables: declaration.Database<Models> | readonly Table[],
) => Promise<TestConnection<Models>>;

/** Key a test database's sensitive columns with the test cipher. */
function keyed<Database extends DatabaseConnection>(database: Database): Database {
    database.key(TEST_CIPHER);

    return database;
}
