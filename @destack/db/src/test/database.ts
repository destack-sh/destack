import { join } from "node:path";
import postgres from "postgres";
import { testHost } from "@destack/test/host";
import { Simulation } from "@destack/environment/simulation";
import type { DatabaseConnection, DatabaseEnvironment } from "../database/connection.ts";
import type { Model } from "../query/model.ts";
import { Dialect } from "../dialect/dialect.ts";
import type { Table } from "../table/table.ts";
import type * as declaration from "../declare/database.ts";
import { declareState } from "../migration/state.ts";
import * as sqlite from "../bun/connection.ts";
import * as postgresql from "../postgres/index.ts";
import { ColumnCipher } from "../database/cipher.ts";
import { LOG_EPOCH, LOG_HORIZON } from "../log/schema.ts";
import { createLog } from "../log/trigger.ts";
import { sql } from "../sql/index.ts";
import type { Channel } from "../channel/channel.ts";
import { channelHub } from "../channel/channel.ts";
import { STATE, type TableState } from "../migration/state.ts";
import { upsertPredefined } from "../migration/plan.ts";
import { inject, onTestFinished, type TestProject, testTarget } from "@destack/test";
import { type JsonValue, mapValues, present, Uuid } from "@destack/schema";
import type {
    ImageIndex,
    ImageSource,
    TemplateDatabases,
    TemplateImage,
    TemplateStorage,
    TemplateTables,
    TestTemplate,
} from "./template.ts";

/** The host the harness keeps files and schemas on, shared by the test processes of a run. */
const HOST = testHost();

/** The node a template's databases run as in its world. */
const TEMPLATE_NODE = "template";

/** The cipher of every test database, a fixed key so schemas reused across test processes open. */
export const TEST_CIPHER = new ColumnCipher(new Uint8Array(32).fill(7), HOST.random);

/** The connections a test process pools, past a world's 18 open transactions: 88 for 4 workers. */
const TEST_POOL_CONNECTIONS = 20;

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
        const address = HOST.process.variables["DESTACK_TEST_POSTGRES"];
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
    static create<Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>>(
        tables: declaration.Database<Models> | readonly Table[],
        options: TestDatabaseOptions,
    ): Promise<TestDatabase<Models>> {
        // count the harness's file and schema work in flight on the database's clock
        return options.environment.clock.trackPeer(TestDatabase.#create(tables, options));
    }

    /** Create an isolated database, as `create` tracks it. */
    static async #create<
        Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
    >(
        tables: declaration.Database<Models> | readonly Table[],
        options: TestDatabaseOptions,
    ): Promise<TestDatabase<Models>> {
        // read the tables and the database, the project's unless given
        const declared = "tables" in tables ? tables.tables : tables;
        const dialect = options.dialect ?? TestDatabase.target();

        // claim or create a PostgreSQL schema, or open SQLite in memory or as a file copy
        if (dialect === "postgresql") {
            return TestDatabase.#postgres(declared, tables, options);
        } else if (options.isMigrated !== true && (options.storage ?? "memory") === "memory") {
            return new TestDatabase(
                keyed(
                    await sqlite.connectBunSqlite(":memory:", tables, {
                        environment: options.environment,
                    }),
                ),
                undefined,
                async () => {},
            );
        }

        return TestDatabase.#sqliteFile(declared, tables, options);
    }

    /** Copy a template's image into a fork of its world at its write's end, with its value. */
    static async copy<Value extends JsonValue, Databases extends TemplateDatabases>(
        template: TestTemplate<Value, Databases>,
        options: TemplateCopyOptions,
    ): Promise<{
        readonly simulation: Simulation;
        readonly databases: TemplateStorage<Databases>;
        readonly value: Value;
    }> {
        // read the template's image in the run, refusing a template no setup of the run lists
        const images = options.images ?? inject("templates");
        const dialect = options.dialect ?? TestDatabase.target();
        const image = images?.[template.name]?.[dialect];
        if (image === undefined) {
            throw new TypeError(
                `the run has no ${dialect} image of template ${template.name}: list it in its package's test templates`,
            );
        }

        // fork the template's world by the label, its databases running as the write's node
        const recorded = { seed: template.world.seed, start: image.end };
        const simulation = Simulation.fork(recorded, options.label);
        onTestFinished(() => simulation[Symbol.asyncDispose]());
        const environment = simulation.environment(TEMPLATE_NODE);

        // copy each database, counting the harness's copying in flight on the fork's clock
        const copied = await environment.clock.trackPeer(
            TestDatabase.#copyDatabases(template, image, environment),
        );
        const databases = mapValues<Databases, TemplateStorage<Databases>>(
            template.databases,
            (_, name) => present(copied.get(name), `the copy's database ${name}`),
        );

        return { simulation, databases, value: template.value.parse(image.value) };
    }

    /** Copy each database of a template's image into a claimed schema or a temporary file. */
    static async #copyDatabases(
        template: Pick<TestTemplate, "name" | "databases">,
        image: TemplateImage,
        environment: DatabaseEnvironment,
    ): Promise<Map<string, TestDatabase>> {
        const copied = await Promise.all(
            Object.entries(template.databases).map(async ([name, tables]) => {
                // refuse an image lacking the database
                const source = image.sources[name];
                if (source === undefined) {
                    throw new TypeError(`the image of template ${template.name} has no ${name}`);
                }

                return [name, await TestDatabase.#copy(tables, source, environment)] as const;
            }),
        );

        return new Map(copied);
    }

    /** Write a template's image once for a run: a file or schema per database the run stores. */
    static async image<Value extends JsonValue, Databases extends TemplateDatabases>(
        template: TestTemplate<Value, Databases>,
        dialect: Dialect,
        run: { readonly id: string; readonly directory: string },
    ): Promise<TemplateImage> {
        // open the template's world, then each database migrated in turn, named by the digest of the template's name and its own
        const simulation = new Simulation(template.world);
        const environment = simulation.environment(TEMPLATE_NODE);
        const opened = new Map<string, ImageDatabase>();
        let value: Value;
        let end: number;
        try {
            for (const [name, tables] of Object.entries(template.databases)) {
                const key = (await digestOf(`${template.name}/${name}`)).slice(0, 16);
                opened.set(
                    name,
                    await TestDatabase.#openImage(tables, dialect, key, run, environment),
                );
            }

            // write the rows into them
            const databases = mapValues<Databases, TemplateStorage<Databases>>(
                template.databases,
                (_, name) => present(opened.get(name), `the image's database ${name}`).storage,
            );
            value = await TestDatabase.#write(template, databases, run.directory, simulation);
            end = simulation.clock.now();
        } catch (error) {
            // close what a failed write opened
            await Promise.all([...opened.values()].map((entry) => entry.storage.close()));
            throw error;
        }

        // close the written databases, then read where each keeps its rows
        await Promise.all([...opened.values()].map((entry) => entry.storage.close()));
        const sources = await Promise.all(
            [...opened].map(async ([name, entry]) => [name, await entry.source()] as const),
        );

        return { sources: Object.fromEntries(sources), value, end };
    }

    /** Copy one database of an image into a claimed schema or a temporary file. */
    static async #copy(
        tables: TemplateTables,
        source: ImageSource,
        environment: DatabaseEnvironment,
    ): Promise<TestDatabase> {
        const declared = "tables" in tables ? tables.tables : tables;

        return source.dialect === "postgresql"
            ? await TestDatabase.#postgres(
                  declared,
                  tables,
                  { isMigrated: true, environment },
                  source,
              )
            : await TestDatabase.#sqliteFile(declared, tables, { environment }, source.file);
    }

    /** Open one database of an image migrated, and read where it keeps its rows once written. */
    static async #openImage(
        tables: TemplateTables,
        dialect: Dialect,
        key: string,
        run: { readonly id: string; readonly directory: string },
        environment: DatabaseEnvironment,
    ): Promise<ImageDatabase> {
        // write it into a migrated schema that stays until the run's end
        const declared = "tables" in tables ? tables.tables : tables;
        if (dialect === "postgresql") {
            // stamp its creation, which stops other runs' sweeps dropping it until it expires
            const server = TestServer.of(TestDatabase.address());
            const schema = `test_t${run.id}_${key}`;
            await server.session.unsafe(`CREATE SCHEMA "${schema}"`);
            await server.session.unsafe(
                `COMMENT ON SCHEMA "${schema}" IS '${HOST.clock.now()} image'`,
            );

            // migrate it, then open it for the template's write
            const open: TestConnector = (connected) =>
                server.connect(schema, connected, environment);
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
            const storage = new TestDatabase(await open(tables), open, async () => {});
            const source = async (): Promise<ImageSource> => ({
                dialect,
                schema,
                columns: await readColumns(server.session, schema),
            });

            return { storage, source };
        }

        // write a migrated file in the run's directory, and checkpoint its WAL into it once written
        const file = join(run.directory, `${key}.db`);
        await HOST.files.copyFile(
            await sqliteTemplate(declared, false, replicatedOf(tables), environment),
            file,
        );
        const source = async (): Promise<ImageSource> => {
            // checkpoint the WAL into the file
            const database = await sqlite.connectBunSqlite(file, tables, { environment });
            await database.executeScript("PRAGMA wal_checkpoint(TRUNCATE)");
            await database.close();

            return { dialect, file };
        };

        // open the file the template writes into, in the template's environment
        const storage = await TestDatabase.#sqliteAt(
            file,
            tables,
            async () => {},
            environment,
            "renew",
        );

        return { storage, source };
    }

    /** Store SQLite in the caller's file, migrated when the file is new. */
    static async at<Models extends Readonly<Record<string, Model>>>(
        file: string,
        tables: declaration.Database<Models> | readonly Table[],
        environment: DatabaseEnvironment,
    ): Promise<TestDatabase<Models>> {
        // copy the migrated database into a new file
        const declared = "tables" in tables ? tables.tables : tables;
        if (!(await HOST.files.exists(file))) {
            await HOST.files.copyFile(
                await sqliteTemplate(declared, false, replicatedOf(tables), environment),
                file,
            );
        }

        return TestDatabase.#sqliteAt(file, tables, async () => {}, environment, "renew");
    }

    /** Drop the expired schemas no test claims and the images of runs that ended long ago. */
    static sweep(): Promise<void> {
        return TestSchema.sweep(TestDatabase.address());
    }

    /** Close the process's pools and claim sessions, releasing its claims. */
    static close(): Promise<void> {
        return TestServer.close();
    }

    /** Write a template into its databases and a directory of the run's, then close them. */
    static async #write<Value extends JsonValue, Databases extends TemplateDatabases>(
        template: TestTemplate<Value, Databases>,
        databases: TemplateStorage<Databases>,
        directory: string,
        simulation: Simulation,
    ): Promise<Value> {
        const files = join(directory, `files-${Uuid.v4(HOST.random)}`);
        await HOST.files.mkdir(files);

        return await template.write(databases, files, simulation);
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
        const { environment } = options;
        if (options.isMigrated === true && declared.length > 0) {
            const state = declareState(declared, "postgresql", {
                isReplica: options.isReplica ?? false,
                replicated: replicatedOf(tables),
            });
            const schema = await TestSchema.claim(address, state, tables, environment, template);
            const open: TestConnector = (connected) => schema.connect(connected, environment);

            return new TestDatabase(await open(tables), open, () => schema.release());
        }

        // create an empty schema for no tables or no migration, dropped once the database closes
        const server = TestServer.of(address);
        const schema = `test_${Uuid.v4(HOST.random).replaceAll("-", "")}`;
        await server.session.unsafe(`CREATE SCHEMA "${schema}"`);
        const open: TestConnector = (connected) => server.connect(schema, connected, environment);

        return new TestDatabase(await open(tables), open, async () => {
            await server.session.unsafe(`DROP SCHEMA "${schema}" CASCADE`);
        });
    }

    /** Store SQLite in a temporary file, copied from a template's or the migrated database. */
    static async #sqliteFile<Models extends Readonly<Record<string, Model>>>(
        declared: readonly Table[],
        tables: declaration.Database<Models> | readonly Table[],
        options: TestDatabaseOptions,
        template?: string,
    ): Promise<TestDatabase<Models>> {
        // copy the template's or the migrated database into a temporary file, each step tracked on the store's clock
        const { environment } = options;
        const { clock } = environment;
        const directory = await clock.trackHost(HOST.files.makeTemporaryDirectory("destack-test-"));
        const file = join(directory, "test.db");
        const source =
            template ??
            (options.isMigrated === true
                ? await clock.trackHost(
                      sqliteTemplate(
                          declared,
                          options.isReplica ?? false,
                          replicatedOf(tables),
                          environment,
                      ),
                  )
                : undefined);
        if (source !== undefined) {
            await clock.trackHost(HOST.files.copyFile(source, file));
        }

        // open it, a world's copy keeping its log epoch
        return TestDatabase.#sqliteAt(
            file,
            tables,
            () => clock.trackHost(HOST.files.remove(directory, { recursive: true })),
            environment,
            template === undefined ? "renew" : "keep",
        );
    }

    /** Store SQLite in a file, sharing one hub, removed as given once closed. */
    static async #sqliteAt<Models extends Readonly<Record<string, Model>>>(
        file: string,
        tables: declaration.Database<Models> | readonly Table[],
        remove: () => Promise<void>,
        environment: DatabaseEnvironment,
        epoch: "renew" | "keep",
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
            const opened = await sqlite.connectBunSqlite(file, connected, {
                openChannel,
                environment,
            });
            await opened.executeScript("PRAGMA synchronous = OFF");

            return keyed(opened);
        };

        // start a new epoch for a new logged database
        const database = await open(tables);
        const [log] = await database.all(
            sql`SELECT name FROM sqlite_master WHERE name = ${LOG_EPOCH}`,
        );
        if (log !== undefined && epoch === "renew") {
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

        // track the connection, counting its opening in flight on the database's clock
        const connection = await this.database.state.clock.trackPeer(this.#open(tables));
        this.#connections.add(connection);

        return connection;
    }

    /** Close every connection and remove the database. */
    async close(): Promise<void> {
        // close the connections, then remove the database on its clock
        await Promise.all([...this.#connections].map((connection) => connection.close()));
        const { clock } = this.database.state;
        await this.database.close();
        await clock.trackPeer(this.#remove());
    }
}

/** Where a template's copy reads its image and how it forks the template's world. */
export interface TemplateCopyOptions {
    /** The label the copy forks the template's world by, distinct per copy in one world. */
    readonly label: string;
    /** The images of the run's templates, the test run's by default. */
    readonly images?: ImageIndex | undefined;
    /** The database the images are of, the test project's by default. */
    readonly dialect?: Dialect;
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
    /** The environment its log stamps changes and draws tokens in, its store's. */
    readonly environment: DatabaseEnvironment;
}

/** One database of an image being written: its storage and its rows' place. */
interface ImageDatabase {
    /** The migrated database the template writes into. */
    readonly storage: TestDatabase;
    /** Read where the database keeps its rows once written and closed. */
    readonly source: () => Promise<ImageSource>;
}

/** A template image's PostgreSQL schema, whose rows claimed schemas copy. */
type PostgresImage = Extract<ImageSource, { readonly dialect: "postgresql" }>;

/** A migrated schema one test database claims, its session with the claim's lock. */
class TestSchema {
    /** The process's server, whose session has the claim's lock. */
    readonly server: TestServer;
    /** The schema's name. */
    readonly name: string;
    /** The relations and applied state the schema was claimed with. */
    readonly shape: TestSchemaShape;
    /** Store a claimed schema with its session and shape. */
    private constructor(server: TestServer, name: string, shape: TestSchemaShape) {
        // store the claim's server, schema and shape
        this.server = server;
        this.name = name;
        this.shape = shape;
    }

    /** Claim a migrated schema reset to a template's rows: a free one, else a new one. */
    static async claim(
        address: string,
        state: readonly TableState[],
        tables: declaration.Database | readonly Table[],
        environment: DatabaseEnvironment,
        template?: PostgresImage,
    ): Promise<TestSchema> {
        // derive the schema names from the digest of the state and the log it migrates beside
        const server = TestServer.of(address);
        const { session } = server;
        const layout = JSON.stringify([state, createLog("postgresql")]);
        const prefix = `test_s${(await digestOf(layout)).slice(0, 16)}_`;

        // claim a free schema of the same state whose shape still matches its stamp
        const listed = await session<{ name: string }[]>`
            SELECT nspname AS name FROM pg_namespace WHERE starts_with(nspname, ${prefix})`;
        for (const { name } of listed) {
            if (await server.claimListed(name)) {
                // read the shape of a schema whose migration finished, which its stamp marks last
                const stamp = await readStamp(session, name);
                const shape = stamp === undefined ? undefined : await readShape(session, name);
                if (shape !== undefined && stamp === (await digestOf(shape.fingerprint))) {
                    const schema = new TestSchema(server, name, shape);
                    await schema.#stamp();
                    await schema.#reset(state, environment, template);

                    return schema;
                }
                await server.drop(name);
            }
        }

        // create and migrate a new schema with its claim time
        const name = `${prefix}${Uuid.v4(HOST.random).replaceAll("-", "")}`;
        await server.claim(name);
        await session.unsafe(`CREATE SCHEMA "${name}"`);
        const database = await server.connect(name, tables, environment);
        await database.apply(await database.plan({ declared: state }));
        await database.close();
        const schema = new TestSchema(server, name, await readShape(session, name));
        await schema.#stamp();
        if (template !== undefined) {
            await schema.#reset(state, environment, template);
        }

        return schema;
    }

    /** Drop the expired schemas no test claims and the templates of runs that ended long ago. */
    static async sweep(address: string): Promise<void> {
        // list the claimed schemas and run templates past retention
        const server = TestServer.of(address);
        const { session } = server;
        const expired = HOST.clock.now() - SCHEMA_RETENTION_MILLISECONDS;
        const schemas = await session<{ name: string }[]>`
            SELECT nspname AS name FROM pg_namespace
            WHERE (starts_with(nspname, 'test_s') OR starts_with(nspname, 'test_t'))
                AND split_part(coalesce(obj_description(oid, 'pg_namespace'), '0'), ' ', 1)::bigint < ${expired}`;

        // drop each one no session has claimed
        for (const { name } of schemas) {
            if (await server.claimListed(name)) {
                await server.drop(name);
            }
        }
    }

    /** Connect to the schema on the process's pool. */
    connect<Models extends Readonly<Record<string, Model>>>(
        tables: declaration.Database<Models> | readonly Table[],
        environment: DatabaseEnvironment,
    ): Promise<TestConnection<Models>> {
        return this.server.connect(this.name, tables, environment);
    }

    /** Release the claim, leaving the schema for the next test of its state. */
    release(): Promise<void> {
        return this.server.release(this.name);
    }

    /** Stamp the schema with its claim time and its shape's digest. */
    async #stamp(): Promise<void> {
        const digest = await digestOf(this.shape.fingerprint);
        await this.server.session.unsafe(
            `COMMENT ON SCHEMA "${this.name}" IS '${HOST.clock.now()} ${digest}'`,
        );
    }

    /** Replace the rows with a template's and its log identity, or start a new epoch. */
    async #reset(
        state: readonly TableState[],
        environment: DatabaseEnvironment,
        template?: PostgresImage,
    ): Promise<void> {
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

        // take the template's log and epoch, else start a new epoch
        const epoch =
            template === undefined
                ? `UPDATE ${quoted(LOG_EPOCH)} SET epoch = '${Uuid.v7(environment)}' WHERE slot = 1;`
                : `UPDATE ${quoted(LOG_EPOCH)} SET (log, epoch) = (SELECT log, epoch FROM "${template.schema}"."${LOG_EPOCH}" WHERE slot = 1) WHERE slot = 1;`;

        // run the batch as one implicit transaction without triggers
        await this.server.session.unsafe(`SET LOCAL session_replication_role = replica;
            SET LOCAL search_path = "${this.name}";
            ${emptied.map((table) => `DELETE FROM ${quoted(table.name)};`).join("\n")}
            ${sequences.map((sequence) => `ALTER SEQUENCE ${quoted(sequence.name)} RESTART;`).join("\n")}
            UPDATE ${quoted(LOG_HORIZON)} SET sequence = 0 WHERE slot = 1;
            ${copied.join("\n")}
            ${epoch}`);
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

/** A test server of this process: one pool for every schema, one session for every claim. */
class TestServer {
    /** The servers of this process, by address. */
    static readonly #servers = new Map<string, TestServer>();
    /** The pool every test database of the process runs on, each in its schema. */
    readonly pool: postgresql.PostgresPool;
    /** The session with the process's schema claims as advisory locks. */
    readonly session: postgres.Sql;
    /** The schemas the process claims, as a session's advisory lock is reentrant. */
    readonly #claimed = new Set<string>();

    /** Open a server's pool and claim session. */
    private constructor(address: string) {
        // commit without waiting for the WAL flush, which a test schema has no use for
        this.pool = new postgresql.PostgresPool(address, HOST, {
            connections: TEST_POOL_CONNECTIONS,
            settings: { synchronous_commit: "off" },
        });
        // open one session for the process's claims, as its advisory locks end with its connection
        this.session = postgres(address, {
            max: 1,
            max_lifetime: null,
            fetch_types: false,
            onnotice: postgresql.reportNotices(HOST.telemetry),
        });
    }

    /** Find or open the process's server at an address. */
    static of(address: string): TestServer {
        // reuse the server opened before
        const known = TestServer.#servers.get(address);
        if (known !== undefined) {
            return known;
        }

        // open the address's server once
        const server = new TestServer(address);
        TestServer.#servers.set(address, server);

        return server;
    }

    /** Close every server of this process: its pool, then its claim session. */
    static async close(): Promise<void> {
        // forget the servers, then end each one's connections
        const servers = [...TestServer.#servers.values()];
        TestServer.#servers.clear();
        for (const server of servers) {
            await server.pool.end();
            await server.session.end();
        }
    }

    /** Connect to a schema on the pool. */
    async connect<Models extends Readonly<Record<string, Model>>>(
        schema: string,
        tables: declaration.Database<Models> | readonly Table[],
        environment: DatabaseEnvironment,
    ): Promise<TestConnection<Models>> {
        return keyed(await postgresql.connectPostgres(this.pool, tables, { environment, schema }));
    }

    /** Claim a schema, reporting whether no other process or test claims it. */
    async claim(name: string): Promise<boolean> {
        // refuse a schema this process claims already, marking it before the lock answers
        if (this.#claimed.has(name)) {
            return false;
        }
        this.#claimed.add(name);

        // take its advisory lock, dropping the mark when the lock fails or another process has it
        let isLocked: unknown;
        try {
            isLocked = (
                await this.session`SELECT pg_try_advisory_lock(hashtext(${name}))`.values()
            )[0]?.[0];
        } finally {
            if (isLocked !== true) {
                this.#claimed.delete(name);
            }
        }
        if (typeof isLocked !== "boolean") {
            throw new TypeError("the advisory lock returned no boolean");
        }

        return isLocked;
    }

    /** Claim a listed schema, reporting whether it was free and still exists. */
    async claimListed(name: string): Promise<boolean> {
        // claim the name, or leave one another test or process claims
        if (!(await this.claim(name))) {
            return false;
        }

        // release a name another session dropped after the listing
        const isPresent: unknown = (
            await this.session`SELECT to_regnamespace(${name}) IS NOT NULL`.values()
        )[0]?.[0];
        if (typeof isPresent !== "boolean") {
            throw new TypeError("the schema lookup returned no boolean");
        } else if (!isPresent) {
            await this.release(name);
        }

        return isPresent;
    }

    /** Release a schema's claim. */
    async release(name: string): Promise<void> {
        // refuse to release a claim the session no longer has
        const isUnlocked: unknown = (
            await this.session`SELECT pg_advisory_unlock(hashtext(${name}))`.values()
        )[0]?.[0];
        if (isUnlocked !== true) {
            throw new TypeError(`the claim of ${name} was lost`);
        }
        this.#claimed.delete(name);
    }

    /** Drop a claimed schema, then release its claim. */
    async drop(name: string): Promise<void> {
        await this.session.unsafe(`DROP SCHEMA "${name}" CASCADE`);
        await this.release(name);
    }
}

/** Digest a text as hexadecimal SHA-256. */
async function digestOf(text: string): Promise<string> {
    const bytes = new TextEncoder().encode(text);

    return new Uint8Array(await HOST.crypto.digest("SHA-256", bytes)).toHex();
}

/** Read a schema's shape digest once its migration finished, absent before. */
async function readStamp(session: postgres.Sql, name: string): Promise<string | undefined> {
    const comment: unknown = (
        await session`SELECT obj_description(to_regnamespace(${name}), 'pg_namespace')`.values()
    )[0]?.[0];

    return typeof comment === "string" ? comment.split(" ")[1] : undefined;
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
    environment: DatabaseEnvironment,
): Promise<string> {
    // name the file by the digest of the state and the log it migrates beside
    const key = JSON.stringify(declareState(tables, "sqlite", { isReplica, replicated }));
    const layout = JSON.stringify([key, createLog("sqlite")]);
    const directory = join(await HOST.files.runtimeDirectory(), "destack-template");
    const file = join(directory, `${await digestOf(layout)}.db`);

    // reuse the file another test migrated for the state and log
    if (await HOST.files.exists(file)) {
        return file;
    }

    // migrate a file beside it and checkpoint its WAL, then move it in whole
    await HOST.files.mkdir(directory, { recursive: true });
    const written = join(directory, `${Uuid.v7(HOST)}.db`);
    const database = await sqlite.connectBunSqlite(written, tables, { environment });
    await database.migrate(tables, { isReplica, replicated });
    await database.executeScript("PRAGMA wal_checkpoint(TRUNCATE)");
    await database.close();
    await HOST.files.rename(written, file);

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
