import { join } from "node:path";
import { type Select, Snapshot, type Table } from "@destack/db";
import { MemoryBlobStorage } from "@destack/resource";
import { pathToFileURL } from "node:url";
import { AccessContext, Authorization, Caller, principal, type StepUp } from "@destack/access";
import { SpaceAddress } from "@destack/account/address";
import type { PlacementEndpoint } from "@destack/account/object";
import {
    CLIENT_KEYS,
    ClientKey,
    type EmailSignIn,
    Issuer,
    type LoginClient,
    NativeSignIn,
    Login,
    SESSIONS,
    SessionStore,
    currentLogin,
} from "@destack/account/native";
import * as native from "@destack/account/native";
import accountPackage, { OAUTH_PATH } from "@destack/account/package";
import { Platform } from "@destack/account/object";
import { connectBunSqlite, openBunSqlite } from "@destack/db/bun";
import type { SqliteDatabase } from "@destack/db/sqlite";
import type { EventStore } from "@destack/event";
import type { Keychain } from "@destack/identity";
import { BunKeychain } from "@destack/identity/bun";
import { call, device, Installation, type ObjectType, tablesOf } from "@destack/object";
import objectPackage from "@destack/object/package";
import { distribution } from "@destack/update/object";
import { implementPackage, ObjectStore, type PackageImplementation } from "@destack/object/server";
import { ObjectClient, type ObjectQueries, type RelationalQuery } from "@destack/object/client";
import { Package, PackageId } from "@destack/package";
import { Identifier, nth, schema } from "@destack/schema";
import { type Fetch, PackageHandle, PackageMount } from "@destack/object";
import { Bookmark, type ClientOptions, createClient } from "@destack/object/client";
import { type CallKey } from "@destack/object";
import { RequestId, Scope, Subject } from "@destack/sync";
import type { Setting } from "@destack/setting";
import { deviceSetting, setting, SettingValue } from "@destack/setting/object";
import settingPackage from "@destack/setting/package";
import machinePackage from "../../package.ts";
import { MachineError } from "../../error/index.ts";
import { Session } from "./session.ts";
import { computer, visit } from "./object/index.ts";
import { Loopback, StageHome } from "../process/index.ts";
import type { SignUp } from "@destack/account/better-auth";
import { type Clock, type Environment, FileSystemError } from "@destack/environment";

/** The file of the device's local objects, in the machine's directory. */
const DATABASE_FILE = "device.db";

/** The file naming the device, beside its database. */
const IDENTITY_FILE = "device.json";

/** The permissions of the device's identity file: the person's alone. */
const IDENTITY_MODE = 0o600;

/** Whether this computer has a machine, and its record while it runs. */
export type MachineStatus =
    | { readonly kind: "absent" }
    | { readonly kind: "stopped" }
    | { readonly kind: "running"; readonly computer: Select<typeof computer.table> };

/** A scope's objects replicated into a replica in memory until closed. */
export class ScopeReplica<
    Objects extends Readonly<Record<string, ObjectType>> = Readonly<Record<string, ObjectType>>,
> {
    /** The replica. */
    readonly client: ObjectClient<Objects>;
    /** Rejects with the first failure and stops the replica. */
    readonly failed: Promise<never>;
    /** The controller that stops following. */
    readonly #stopping: AbortController;
    /** The replica's database in memory. */
    readonly #database: Pick<SqliteDatabase, "close">;

    /** Take a replica following its scope until stopped. */
    constructor(fields: {
        readonly client: ObjectClient<Objects>;
        readonly failed: Promise<never>;
        readonly stopping: AbortController;
        readonly database: Pick<SqliteDatabase, "close">;
    }) {
        // take the client and its failure, then what stops and releases it
        this.client = fields.client;
        this.failed = fields.failed;
        this.#stopping = fields.stopping;
        this.#database = fields.database;
    }

    /** Aborts once the replica stops following. */
    get signal(): AbortSignal {
        return this.#stopping.signal;
    }

    /** Read a query of the replica once and close it. */
    async read<Value>(
        select: (query: ObjectQueries<Objects>) => RelationalQuery<Value>,
    ): Promise<Value> {
        try {
            // wait for the value or the first failure
            return await Promise.race([select(this.client.query), this.failed]);
        } finally {
            await this.close();
        }
    }

    /** Yield a query's value at every change, until the replica stops or fails. */
    async *watch<Value>(
        select: (query: ObjectQueries<Objects>) => RelationalQuery<Value>,
    ): AsyncGenerator<Value> {
        const live = select(this.client.query).subscribe();
        try {
            // yield each change once the value arrived
            await Promise.race([live.ready, this.failed]);
            for await (const value of live.watch(this.signal)) {
                yield value;
            }

            // fail with the replica's failure, ending quietly once it closed
            await Promise.race([this.failed, Promise.resolve()]);
        } finally {
            await live.close();
        }
    }

    /** Stop following and release the replica. */
    async close(): Promise<void> {
        this.#stopping.abort();
        await this.client.close();
        await this.#database.close();
    }
}

/** A device's identity, as its identity file names it. */
const DeviceIdentity = schema.object({
    /** The device's identifier, the principal its local objects partition by. */
    id: schema.identifier("device"),
});

/** What a device runs with. */
export interface DeviceOptions {
    /** The directory with the machine's state and the device's database. */
    readonly home: string;
    /** The clock, network, files, platform and server the device runs on. */
    readonly environment: Environment<"files" | "host" | "server">;
    /** The origin of the universe people sign in at. */
    readonly issuer: string;
    /** Open the keychain of a service's secrets. */
    readonly keychain: (service: string) => Keychain;
}

/** This computer's device: its identity, local objects, people's sessions and client verbs. */
export class Device implements AsyncDisposable {
    /** The package the device's store serves its local objects as: the machine's. */
    static readonly AUDIENCE: PackageId = machinePackage.id;
    /** The directory with the machine's state and the device's database. */
    readonly home: string;
    /** The owner's loopback connection to this computer's running machine. */
    readonly loopback: Loopback;
    /** The positions this device's calls saw, which its later calls wait for: read your writes. */
    readonly bookmark = new Bookmark();
    /** The device, the partition of its local objects. */
    readonly subject: Subject;
    /** The device's database of local objects, shared by its processes. */
    readonly database: SqliteDatabase;
    /** The people signed in on this computer. */
    readonly logins: LoginClient;
    /** The values of client settings this device stores. */
    readonly settings: ObjectClient<{ readonly deviceSetting: typeof deviceSetting }>;
    /** The objects the people signed in on this computer opened. */
    readonly visits: ObjectClient<{ readonly visit: typeof visit }>;
    /** The keychain's sessions of the people signed in. */
    readonly sessions: SessionStore;
    /** The keys of this computer's devices, which prove a person's client to the machine. */
    readonly keys: ClientKey;
    /** Sign people in on this device. */
    readonly signIns: NativeSignIn;
    /** The fetch calling the issuer and the machines. */
    readonly #fetch: Fetch;
    /** The clock, network, files, platform and server the device runs on. */
    readonly environment: Environment<"files" | "host" | "server">;

    /** Take a device's local objects and sessions. */
    private constructor(
        options: DeviceOptions,
        fields: Pick<
            Device,
            "subject" | "database" | "logins" | "settings" | "visits" | "sessions" | "keys"
        >,
    ) {
        // set the device's state and sign people in with its issuer
        this.home = options.home;
        this.loopback = new Loopback(options.home, options.environment, this.bookmark);
        this.subject = fields.subject;
        this.database = fields.database;
        this.logins = fields.logins;
        this.settings = fields.settings;
        this.visits = fields.visits;
        this.sessions = fields.sessions;
        this.keys = fields.keys;
        this.#fetch = (request) => options.environment.network.fetch(request);
        this.environment = options.environment;

        // register the computer's operating system with its client, when Client Hints list it
        const host = options.environment.host;
        const system = Platform.ofNode(host.os);
        this.signIns = new NativeSignIn({
            issuer: new Issuer(
                `${PackageMount.url(options.issuer, accountPackage.id)}${OAUTH_PATH}`,
                this.#fetch,
                options.environment,
                this.bookmark,
            ),
            sessions: fields.sessions,
            keys: fields.keys,
            logins: fields.logins,
            environment: options.environment,
            client: {
                name: host.hostname,
                ...(system === undefined ? {} : { platform: system }),
                architecture: host.architecture,
                version: import.meta.destack.package.version,
            },
        });
    }

    /** Open this release channel's device in an environment, with the system's keychains. */
    static current(
        environment: Environment<"files" | "process" | "host" | "server">,
    ): Promise<Device> {
        const home = StageHome.current(environment);

        return Device.open({
            home: home.directory,
            environment,
            issuer: home.issuer,
            keychain: (service) => new BunKeychain(service, environment.files),
        });
    }

    /** Create the device and its local objects at the machine's first start, then open it. */
    static async create(
        options: DeviceOptions,
        objects: Readonly<Record<string, ObjectType>>,
    ): Promise<Device> {
        // name the device once
        const identity = join(options.home, IDENTITY_FILE);
        const created = JSON.stringify({ id: Identifier.create("device", options.environment) });
        await options.environment.files
            .writeTextFile(identity, created, { createNew: true, mode: IDENTITY_MODE })
            .catch((error: unknown) => {
                if (!FileSystemError.is(error, "ALREADY_EXISTS")) {
                    throw error;
                }
            });

        // migrate the local objects' tables beside the device store's own
        const { database } = await Device.#file(options.home, options.environment);
        try {
            await ObjectClient.migrate(database, objects, Device.#stored());
        } finally {
            await database.close();
        }

        return Device.open(options);
    }

    /** Forget a device's sessions, client keys and machine keys, as uninstalls do. */
    static async forget(
        home: string,
        keychain: (service: string) => Keychain,
        machines: readonly string[],
        environment: Environment<"files" | "server">,
    ): Promise<void> {
        // forget nothing on a computer whose machine never created its device
        if (!(await environment.files.exists(join(home, DATABASE_FILE)))) {
            return;
        }

        // forget each login's session and its client's key
        const { database } = await Device.#file(home, environment);
        try {
            const logins = await database
                .select({
                    issuer: native.login.table.issuer,
                    subjectId: native.login.table.subjectId,
                    registration: native.login.table.clientId,
                })
                .from(native.login.table);
            const sessions = new SessionStore(keychain(SESSIONS));
            const keys = keychain(CLIENT_KEYS);
            for (const { issuer, subjectId, registration } of logins) {
                await sessions.forget({ issuer, subjectId });
                await keys.remove(registration);
            }

            // forget the keys of each machine the computer's records name, in each machine keychain
            const recorded = await database.select({ id: computer.table.id }).from(computer.table);
            for (const { id } of recorded) {
                for (const service of machines) {
                    await keychain(service).remove(id);
                }
            }
        } finally {
            await database.close();
        }
    }

    /** Open the device its machine created: its local objects, logins, settings and visits. */
    static async open(options: DeviceOptions): Promise<Device> {
        // open the shared database and its logins, settings and visits
        const { environment } = options;
        const { subject, database } = await Device.#file(options.home, environment);
        const logins = await Device.#objects(
            subject,
            database,
            { login: native.login },
            environment,
        );
        const settings = await Device.#objects(subject, database, { deviceSetting }, environment);
        const visits = await Device.#objects(subject, database, { visit }, environment);

        return new Device(options, {
            subject,
            database,
            logins,
            settings,
            visits,
            sessions: new SessionStore(options.keychain(SESSIONS)),
            keys: new ClientKey(options.keychain(CLIENT_KEYS), environment),
        });
    }

    /** Open the device's identity and shared database, refusing before its machine created them. */
    static async #file(
        home: string,
        environment: Environment<"files" | "server">,
    ): Promise<{ readonly subject: Subject; readonly database: SqliteDatabase }> {
        // read the device's identity
        const text = await environment.files
            .readTextFile(join(home, IDENTITY_FILE))
            .catch((error: unknown) => {
                if (!FileSystemError.is(error, "NOT_FOUND")) {
                    throw error;
                }
                throw MachineError.of(
                    "MACHINE_STOPPED",
                    "the Destack machine has not started on this computer",
                    { cause: error },
                );
            });
        const { id } = DeviceIdentity.parse(JSON.parse(text));

        // open the shared file below the device's directory, talking to its other processes on its channel
        const file = new URL(DATABASE_FILE, pathToFileURL(`${home}/`)).href;

        return {
            subject: principal.device.reference(Scope.universe.id, id),
            database: await openBunSqlite({ reference: file }, Device.#stored(), environment),
        };
    }

    /** List the tables of the device's store: the system's, the computer and its distribution. */
    static #stored(): readonly Table[] {
        return tablesOf({
            packages: [objectPackage, { objects: { computer, distribution }, tables: [] }],
            follows: [],
            tables: call.tables,
        }).tables;
    }

    /** Open a client of some local object types as a device over its database. */
    static #objects<const Objects extends Readonly<Record<string, ObjectType>>>(
        subject: Subject,
        database: SqliteDatabase,
        objects: Objects,
        environment: Environment,
    ): Promise<ObjectClient<Objects>> {
        return ObjectClient.open({
            environment,
            database,
            objects,
            scope: subject.id,
            caller: subject,
            device: subject,
            isMigrated: true,
            endpoint: {
                url: "http://local.invalid/",
                fetch: async () => {
                    throw MachineError.of(
                        "METHOD_UNSUPPORTED",
                        "local objects cannot call a server",
                    );
                },
            },
            reconnect: () => {
                throw MachineError.of(
                    "METHOD_UNSUPPORTED",
                    "local objects cannot move to a machine",
                );
            },
        });
    }

    /** The device's identifier. */
    get id(): Identifier<"device"> {
        return schema.identifier("device").parse(this.subject.id);
    }

    /** The clock the device runs at. */
    get clock(): Clock {
        return this.environment.clock;
    }

    /** Open a client of some local object types as the device. */
    objects<const Objects extends Readonly<Record<string, ObjectType>>>(
        objects: Objects,
    ): Promise<ObjectClient<Objects>> {
        return Device.#objects(this.subject, this.database, objects, this.environment);
    }

    /** Implement local types of other packages at the device's store, one server per package. */
    servers(objects: readonly ObjectType[]): PackageImplementation[] {
        return [...Map.groupBy(objects, (type) => type.package.id).values()].map((types) => {
            const owner = nth(types, 0).package;
            const handle = PackageHandle.of(owner, {
                objects: Object.fromEntries(types.map((type) => [type.name, type])),
            });

            return implementPackage(handle, { objects: types });
        });
    }

    /** Open the device's own store over its servers, home of its partition. */
    async store(
        servers: readonly PackageImplementation[],
        callKey: CallKey,
        events: EventStore,
    ): Promise<ObjectStore> {
        // serve each package as its base installation in the device's partition
        const { clock } = this;
        const { crypto } = this.environment;
        const based = await Promise.all(servers.map((server) => server.base(this.subject, crypto)));
        const store = new ObjectStore({
            blobs: new MemoryBlobStorage(crypto),
            servers: based,
            chain: async () => undefined,
            database: this.database,
            principal: this.subject,
            partitions: [this.subject.id],
            callKey,
            events,
            environment: this.environment,
            origin: { package: Package.of(machinePackage) },
        });

        // receive the device's scope once, a root no other home knows, with its base installations
        const now = clock.now();
        const packages = servers.map((server) => server.package);
        await Authorization.run(
            store.authorizer,
            this.database,
            () => AccessContext.of(Caller.of(this.subject), now),
            this.environment,
            async (authorization) => {
                const [received] = await Scope.read(authorization.transaction.view(), [
                    this.subject.id,
                ]);
                if (received === undefined) {
                    await Installation.receive(authorization, this.subject, [], packages);
                }
            },
        );

        return store;
    }

    /** Inspect existing local state without creating files or starting the machine. */
    static async inspect(home: string, environment: Environment<"files">): Promise<MachineStatus> {
        // tell a computer without a machine from one whose machine is stopped
        if (!(await environment.files.exists(join(home, DATABASE_FILE)))) {
            return { kind: "absent" };
        }

        // read the running machine's record, refusing any failure but a stopped machine
        try {
            return {
                kind: "running",
                computer: await new Loopback(home, environment, new Bookmark()).computer(),
            };
        } catch (error) {
            if (MachineError.is(error, "MACHINE_STOPPED")) {
                return { kind: "stopped" };
            }
            throw error;
        }
    }

    /** Create a client of the machine's own package, as its owner. */
    machine() {
        return this.loopback.connect(machinePackage);
    }

    /** Sign a person in through the browser, and enroll an unenrolled machine in their account. */
    async signIn(
        open: (url: string) => Promise<void>,
        signal?: AbortSignal,
        stepUp?: StepUp,
    ): Promise<Login["row"]> {
        const signedIn = await this.signIns.browser(open, stepUp, signal);
        await this.#enroll(signedIn);

        return signedIn;
    }

    /** Sign a person in with a device code, and enroll an unenrolled machine in their account. */
    async signInWithCode(
        show: Parameters<NativeSignIn["device"]>[0],
        signal?: AbortSignal,
    ): Promise<Login["row"]> {
        const signedIn = await this.signIns.device(show, signal);
        await this.#enroll(signedIn);

        return signedIn;
    }

    /** Sign a person in with a mailed code, and enroll an unenrolled machine in their account. */
    async signInWithEmail(
        email: string,
        ask: (started: EmailSignIn) => Promise<string>,
        signUp?: schema.Input<typeof SignUp>,
    ): Promise<Login["row"]> {
        const signedIn = await this.signIns.email(email, ask, signUp);
        await this.#enroll(signedIn);

        return signedIn;
    }

    /** Sign a person out on this computer, revoking this computer's client and their session. */
    async signOut(login: Login): Promise<void> {
        // revoke the client the person's sign-in registered, which ends its sessions there
        await login.accounts().client.revoke({
            userId: schema.identifier("user").parse(login.row.subjectId),
            id: schema.identifier("client").parse(login.row.clientId),
            requestId: RequestId.create(this.environment),
        });

        // forget the session and the login
        await this.signIns.signOut(login.row);
    }

    /** Read the person this computer acts as. */
    async login(): Promise<Login["row"] | undefined> {
        // read the signed-in people and the one the login setting names
        const [items, chosen] = await Promise.all([
            this.logins.query.login.findMany(),
            this.setting(currentLogin),
        ]);
        const [signedIn, other] = items;

        // take the signed-in person the setting names
        if (chosen !== null) {
            const found = items.find((item) => item.id === chosen);
            if (found === undefined) {
                throw MachineError.of(
                    "LOGIN_NOT_SIGNED_IN",
                    `the login setting picks ${chosen}, who is not signed in`,
                );
            }

            return found;
        }
        // refuse several people the setting chooses none of
        else if (other !== undefined) {
            throw MachineError.of(
                "PERSON_AMBIGUOUS",
                "several people are signed in: choose one with the login setting",
            );
        }

        return signedIn;
    }

    /** Resolve a setting on this computer. */
    async setting<Value extends schema.Schema>(
        declared: Setting<Value>,
    ): Promise<schema.Infer<Value>> {
        const { scope } = declared.definition;

        // read the person's values along their scope chain at the universe
        if (scope === "user") {
            const signedIn = await this.login();
            if (signedIn === undefined) {
                throw MachineError.of("SIGN_IN_REQUIRED", "nobody is signed in");
            }

            return this.#userSetting(declared, (await this.session(signedIn.id)).login);
        }
        // read the values this device stores
        else if (scope === "device") {
            const stored = await this.settings.query.deviceSetting.findMany({
                where: declared.reference,
            });
            const selection = { scope: this.id };

            return declared.resolve(selection, [], [this.id], stored).value;
        }

        throw MachineError.of(
            "SETTING_SCOPE_MISMATCH",
            `a device can only resolve device and person settings, but ${declared.name} is a ${scope} setting`,
        );
    }

    /** Set a setting on this computer, replacing the value set before. */
    async set<Value extends schema.Schema>(
        declared: Setting<Value>,
        value: schema.Infer<Value>,
    ): Promise<void> {
        // read the setting's scope and the value as it is written
        const { scope } = declared.definition;
        const written = { value: schema.json().parse(value), release: declared.package.version };

        // store a device setting's value in this device
        if (scope === "device") {
            const [before] = await this.settings.query.deviceSetting.findMany({
                where: declared.reference,
            });
            const submitted =
                before === undefined
                    ? this.settings
                          .mutate(deviceSetting)
                          .create({ ...declared.reference, ...written })
                    : this.settings.mutate(deviceSetting).update({ id: before.id, ...written });
            await submitted.confirmed;

            return;
        }
        // refuse any other scope but a person's
        else if (scope !== "user") {
            throw MachineError.of(
                "SETTING_SCOPE_MISMATCH",
                `a device can only set device and person settings, but ${declared.name} is a ${scope} setting`,
            );
        }

        // find the person's value set before in their scope at their account
        const signedIn = await this.login();
        if (signedIn === undefined) {
            throw MachineError.of("SIGN_IN_REQUIRED", "nobody is signed in");
        }
        const { login } = await this.session(signedIn.id);
        const values = createClient(
            settingPackage,
            login.universe(settingPackage.id),
            this.environment,
        );
        const placed = signedIn.subjectId;
        const { items } = await values.setting.list({
            scope: placed,
            where: { ...declared.reference, scope: placed },
        });
        const [before] = items;

        // replace it or set the first value
        if (before === undefined) {
            await values.setting.create({
                ...declared.reference,
                ...written,
                scope: placed,
                requestId: RequestId.create(this.environment),
                mode: "set",
            });
        }
        // change the value set before
        else {
            await values.setting.update({
                ...written,
                id: before.id,
                scope: placed,
                requestId: RequestId.create(this.environment),
                revision: before.revision,
            });
        }
    }

    /** Follow some of the machine's own objects, such as its checkouts, as its owner. */
    async follow<const Objects extends Readonly<Record<string, ObjectType>>>(
        objects: Objects,
    ): Promise<ScopeReplica<Objects>> {
        // require one package's objects in their scope's partition
        const { id } = await this.loopback.computer();
        const [first] = Object.values(objects);
        if (first === undefined) {
            throw MachineError.of("OBJECTS_MISSING", "no objects to follow");
        }
        const isLocal = first.scopes.some((type) => type.name === device.name);

        return this.replicate(
            isLocal ? this.id : id,
            objects,
            principal.machine.reference(Scope.universe.id, id),
            this.loopback.options(first.package.id),
            (machine) => {
                throw MachineError.of(
                    "OBJECTS_MOVED",
                    `this computer's objects moved to ${machine}`,
                );
            },
        );
    }

    /** Act as a signed-in person through the session in the keychain, once per sign-in. */
    async session(loginId: Identifier<"login">): Promise<Session> {
        // act through the client the person's current sign-in registered
        const signedIn = await this.#login(loginId);
        const login = new Login(
            signedIn,
            this.sessions,
            this.#fetch,
            this.environment,
            this.bookmark,
        );

        return new Session(this, login);
    }

    /** Read the account this computer's machine is enrolled in, refusing an unenrolled one. */
    async account(): Promise<Identifier<"account">> {
        const { enrollment } = await this.loopback.computer();
        if (enrollment?.status !== "enrolled" || enrollment.kind !== "personal") {
            throw MachineError.of(
                "ENROLLMENT_REQUIRED",
                "this machine is not enrolled in a personal account: sign in to enroll it",
            );
        }

        return enrollment.accountId;
    }

    /** Find where a space lives by its identifier or address, as this computer's machine has it. */
    async locate(space: Identifier<"space"> | SpaceAddress): Promise<PlacementEndpoint> {
        // place the space through the machine's replica of the directory
        const recorded = await this.loopback.computer();
        const located = await this.machine().computer.locate({
            deviceId: recorded.scope,
            id: recorded.id,
            space,
        });

        // call a space of this computer's own machine on loopback
        return located.placement.machine === recorded.id
            ? { ...located, url: await this.origin() }
            : located;
    }

    /** Find the account claiming a handle, as this computer's machine has it. */
    async resolve(handle: string): Promise<Identifier<"account">> {
        const recorded = await this.loopback.computer();
        const { id } = await this.machine().computer.resolve({
            deviceId: recorded.scope,
            id: recorded.id,
            handle,
        });

        return id;
    }

    /** The origin this computer's machine answers at on loopback. */
    async origin(): Promise<string> {
        const { port } = await this.loopback.read();

        return `http://127.0.0.1:${port}`;
    }

    /** Close the device's clients and its database. */
    async [Symbol.asyncDispose](): Promise<void> {
        // close the clients, then the database they share
        await this.logins.close();
        await this.settings.close();
        await this.visits.close();
        await this.database[Symbol.asyncDispose]();
    }

    /** Read a signed-in person's login, refusing one signed out. */
    async #login(loginId: Identifier<"login">): Promise<Login["row"]> {
        const signedIn = await this.logins.query.login.findFirst({ where: { id: loginId } });
        if (signedIn === undefined) {
            throw MachineError.of("SIGN_IN_REQUIRED", "the person is not signed in");
        }

        return signedIn;
    }

    /** Enroll a machine that is not enrolled in the signed-in person's own account. */
    async #enroll(signedIn: Login["row"]): Promise<void> {
        // leave an enrolled machine
        const recorded = await this.loopback.computer();
        if (recorded.enrollment?.status === "enrolled") {
            return;
        }

        // enroll it in the person's own account as the person
        const { login } = await this.session(signedIn.id);
        await this.machine().computer.enroll({
            deviceId: recorded.scope,
            requestId: RequestId.create(this.environment),
            id: recorded.id,
            login: signedIn.id,
            accountId: await login.account(),
        });
    }

    /** Follow a scope's objects into a replica in memory until closed or the first failure. */
    async replicate<const Objects extends Readonly<Record<string, ObjectType>>>(
        scope: string,
        objects: Objects,
        caller: Subject,
        endpoint: ClientOptions,
        reconnect: (machine: Identifier<"machine">) => ClientOptions,
        homes?: (scope: string) => ClientOptions,
    ): Promise<ScopeReplica<Objects>> {
        // store the objects in memory
        const database = await connectBunSqlite(":memory:", ObjectClient.tables(objects), {
            environment: this.environment,
        });
        const client = await ObjectClient.open({
            environment: this.environment,
            database,
            objects,
            scope,
            caller,
            endpoint,
            reconnect,
            ...(homes === undefined ? {} : { homes }),
        });

        // stop following at the first failure
        const stopping = new AbortController();
        const failed = Promise.withResolvers<never>();
        failed.promise.catch(() => {});
        const fail = (error: unknown) => {
            failed.reject(error);
            stopping.abort();
        };
        void client.follow(stopping.signal, fail).catch(fail);

        return new ScopeReplica({ client, failed: failed.promise, stopping, database });
    }

    /** Resolve a person's setting from a replica of their values at the universe. */
    async #userSetting<Value extends schema.Schema>(
        declared: Setting<Value>,
        person: Login,
    ): Promise<schema.Infer<Value>> {
        // follow the person's values at the universe as the person
        const scope = person.subject.id;
        const replica = await this.replicate(
            scope,
            { setting },
            person.subject,
            person.universe(settingPackage.id),
            (machine) => {
                throw MachineError.of(
                    "OBJECTS_MOVED",
                    `the settings of ${scope} cannot move, but moved to ${machine}`,
                );
            },
        );
        try {
            // wait for the values and the access rows of the person's chain
            const { client } = replica;
            const live = client.query.setting.findMany({}).subscribe();
            await Promise.race([live.ready, replica.failed]);
            await live.close();
            const isAccessed = async () =>
                (await client.access.position(client.database)) !== undefined;
            await Promise.race([
                client.database.log.waitUntil(isAccessed, replica.signal),
                replica.failed,
            ]);

            // resolve the value along the chain
            return await SettingValue.resolve(Snapshot.live(client.database), declared, { scope });
        } finally {
            await replica.close();
        }
    }
}
