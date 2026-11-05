import { join } from "node:path";
import { type Select, type Table } from "@destack/db";
import { MemoryBlobStorage } from "@destack/resource";
import { pathToFileURL } from "node:url";
import { AccessContext, Authorization, Caller, principal } from "@destack/access";
import { SpaceAddress } from "@destack/account/address";
import type { PlacementEndpoint } from "@destack/account/object";
import {
    CLIENT_KEYS,
    ClientKey,
    Issuer,
    type LoginClient,
    type LoginCredential,
    NativeClient,
    type NativeClientOptions,
    Login,
    SESSIONS,
    SessionStore,
    currentLogin,
} from "@destack/account/native";
import * as native from "@destack/account/native";
import accountPackage, { OAUTH_PATH } from "@destack/account/package";
import { Platform, type Residency } from "@destack/account/object";
import { connectBunSqlite, openBunSqlite } from "@destack/db/bun";
import type { SqliteDatabase } from "@destack/db/sqlite";
import type { EventStore } from "@destack/event";
import type { Keychain } from "@destack/identity";
import { call, device, Installation, type ObjectType, tablesOf } from "@destack/object";
import objectPackage from "@destack/object/package";
import { distribution } from "@destack/update/object";
import { implementPackage, ObjectStore, type PackageImplementation } from "@destack/object/server";
import { ObjectClient } from "@destack/object/client";
import { Package, PackageId } from "@destack/package";
import { Identifier, nth, present, schema } from "@destack/schema";
import { type Fetch, PackageHandle, PackageMount } from "@destack/object";
import { Bookmark, createClient } from "@destack/object/client";
import { type CallKey } from "@destack/object";
import { RequestId, Scope, Subject } from "@destack/sync";
import type { Setting } from "@destack/setting";
import { deviceSetting } from "@destack/setting/object";
import settingPackage from "@destack/setting/package";
import machinePackage from "../../package.ts";
import { MachineError } from "../../error/index.ts";
import { Session } from "./session.ts";
import { MachineCredential } from "./credential.ts";
import { computer, signIn, type SignInMethod, type SignInState, visit } from "./object/index.ts";
import { Loopback, StageHome } from "../process/index.ts";
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
}

/** This computer's device: its identity, local objects, people's credentials and client verbs. */
export class Device<
    Credential extends LoginCredential = LoginCredential,
> implements AsyncDisposable {
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
    /** The issuer people sign in with. */
    readonly issuer: Issuer;
    /** The people's credentials: the keychain in the machine, the machine in its clients. */
    readonly credential: Credential;
    /** The fetch calling the issuer and the machines. */
    readonly #fetch: Fetch;
    /** The clock, network, files, platform and server the device runs on. */
    readonly environment: Environment<"files" | "host" | "server">;

    /** Take a device's local objects, and build the credential of its people. */
    private constructor(
        options: DeviceOptions,
        fields: Pick<Device, "subject" | "database" | "logins" | "settings" | "visits">,
        credential: (device: Device) => Credential,
    ) {
        // set the device's state and its issuer
        this.home = options.home;
        this.loopback = new Loopback(options.home, options.environment, this.bookmark);
        this.subject = fields.subject;
        this.database = fields.database;
        this.logins = fields.logins;
        this.settings = fields.settings;
        this.visits = fields.visits;
        this.#fetch = (request) => options.environment.network.fetch(request);
        this.environment = options.environment;
        this.issuer = new Issuer(
            `${PackageMount.url(options.issuer, accountPackage.id)}${OAUTH_PATH}`,
            this.#fetch,
            options.environment,
            this.bookmark,
        );

        // build the people's credential
        this.credential = credential(this);
    }

    /** Open this release channel's device in an environment, as a client of its machine. */
    static current(
        environment: Environment<"files" | "process" | "host" | "server">,
    ): Promise<Device<MachineCredential>> {
        const home = StageHome.current(environment);

        return Device.open({ home: home.directory, environment, issuer: home.issuer });
    }

    /** Create the device and its local objects at the machine's first start, with the keychain. */
    static async create(
        options: DeviceOptions,
        objects: Readonly<Record<string, ObjectType>>,
        keychain: (service: string) => Keychain,
    ): Promise<Device<NativeClient>> {
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

        // sign people in with the keychain's sessions and client keys
        const { environment } = options;

        return Device.#open(
            options,
            (opened) =>
                new NativeClient({
                    issuer: opened.issuer,
                    sessions: new SessionStore(keychain(SESSIONS)),
                    keys: new ClientKey(keychain(CLIENT_KEYS), environment),
                    logins: opened.logins,
                    environment,
                    client: Device.#registration(environment),
                }),
        );
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
            const recorded = await database
                .select({ id: computer.table.machineId })
                .from(computer.table);
            for (const { id } of recorded) {
                for (const service of machines) {
                    await keychain(service).remove(id);
                }
            }
        } finally {
            await database.close();
        }
    }

    /** Open the device its machine created, asking the machine for its people's credentials. */
    static open(options: DeviceOptions): Promise<Device<MachineCredential>> {
        return Device.#open(options, (opened) => new MachineCredential(opened.loopback));
    }

    /** Open the device its machine created: its local objects, logins, settings and visits. */
    static async #open<Credential extends LoginCredential>(
        options: DeviceOptions,
        credential: (device: Device) => Credential,
    ): Promise<Device<Credential>> {
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

        return new Device(options, { subject, database, logins, settings, visits }, credential);
    }

    /** Describe the client a sign-in here registers, with its system when Client Hints list it. */
    static #registration(environment: Environment<"host">): NativeClientOptions["client"] {
        const { host } = environment;
        const system = Platform.ofNode(host.os);

        return {
            name: host.hostname,
            ...(system === undefined ? {} : { platform: system }),
            architecture: host.architecture,
            version: import.meta.destack.package.version,
        };
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

    /** Sign a person in through the machine, showing each step it writes until it signs them in. */
    async signIn(
        method: SignInMethod,
        show: (state: SignInState, answer: (code: string) => Promise<void>) => Promise<void>,
        signal?: AbortSignal,
    ): Promise<Login["row"]> {
        // start the sign-in the machine carries out, removed once read
        await using stack = new AsyncDisposableStack();
        const local = await this.objects({ signIn });
        stack.defer(() => local.close());
        const { id } = await local.mutate(signIn).start({ method }).predicted;
        stack.defer(async () => {
            await local.mutate(signIn).remove({ id }).predicted;
        });

        // show each step once as the machine writes it, until it signs the person in or fails
        const watching = new AbortController();
        stack.defer(() => watching.abort());
        const answer = async (code: string) => {
            await local.mutate(signIn).answer({ id, code }).predicted;
        };
        const shown = new Set<string>();
        const until =
            signal === undefined ? watching.signal : AbortSignal.any([watching.signal, signal]);
        for await (const row of local.query.signIn.findFirst({ where: { id } }).watch(until)) {
            const { state } = present(row, `sign-in ${id}`);

            // read the login the machine signed in
            if (state.kind === "signedIn") {
                const found = await this.logins.query.login.findFirst({
                    where: { id: state.login },
                });

                return present(found, `login ${state.login}`);
            }
            // refuse a failed sign-in
            else if (state.kind === "failed") {
                throw MachineError.of("SIGN_IN_FAILED", state.reason);
            }
            // show a step the first time the machine writes it
            else if (!shown.has(state.kind)) {
                shown.add(state.kind);
                await show(state, answer);
            }
        }

        // refuse a sign-in removed or stopped before it ended
        throw MachineError.of("SIGN_IN_FAILED", `sign-in ${id} ended before it signed anyone in`);
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
        await this.credential.forget(login.row);
    }

    /** List the residencies a new person's account may store data in. */
    residencies(): Promise<Pick<Residency, "code" | "name">[]> {
        return this.issuer.residencies();
    }

    /** Read the person this computer acts as. */
    async login(): Promise<Login["row"] | undefined> {
        const [signedIn, values] = await Promise.all([
            this.logins.query.login.findMany(),
            this.settings.query.deviceSetting.findMany({ where: currentLogin.reference }),
        ]);

        return this.choose(signedIn, values);
    }

    /** Pick the person this computer acts as among those signed in, as the login setting says. */
    choose(
        signedIn: readonly Login["row"][],
        values: readonly Select<typeof deviceSetting.table>[],
    ): Login["row"] | undefined {
        // resolve the login setting from this device's values
        const chosen = currentLogin.resolve({ scope: this.id }, [], [this.id], values).value;
        const [first, other] = signedIn;

        // take the signed-in person the setting names
        if (chosen !== null) {
            const found = signedIn.find((item) => item.id === chosen);
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
        // take the one person signed in, none without one
        else {
            return first;
        }
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

            return (await this.session(signedIn.id)).setting(declared);
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
        stack: AsyncDisposableStack,
    ): Promise<ObjectClient<Objects>> {
        // require one package's objects in their scope's partition
        const { id } = await this.loopback.computer();
        const [first] = Object.values(objects);
        if (first === undefined) {
            throw MachineError.of("OBJECTS_MISSING", "no objects to follow");
        }
        const isLocal = first.scopes.some((type) => type.name === device.name);

        // store the objects in memory, closed after the client
        const database = await connectBunSqlite(":memory:", ObjectClient.tables(objects), {
            environment: this.environment,
        });
        stack.defer(() => database.close());

        // follow them as the machine until the stack closes
        return ObjectClient.start(
            {
                environment: this.environment,
                database,
                objects,
                scope: isLocal ? this.id : id,
                caller: principal.machine.reference(Scope.universe.id, id),
                endpoint: this.loopback.options(first.package.id),
                reconnect: (machine) => {
                    throw MachineError.of(
                        "OBJECTS_MOVED",
                        `this computer's objects moved to ${machine}`,
                    );
                },
            },
            stack,
        );
    }

    /** Act as a signed-in person through the session in the keychain, once per sign-in. */
    async session(loginId: Identifier<"login">): Promise<Session> {
        // act through the client the person's current sign-in registered
        const signedIn = await this.#login(loginId);
        const login = new Login(
            signedIn,
            this.credential,
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
        return located.placement.machine === recorded.machineId
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
    async enroll(signedIn: Login["row"]): Promise<void> {
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
}
