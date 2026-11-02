import { fileURLToPath } from "node:url";
import { principal } from "@destack/access";
import type { Login } from "@destack/account/native";
import type { CompilerPool } from "@destack/build";
import { testStore } from "@destack/build/test";
import type { DatabaseConnection } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { type Clock, type Environment, World, type WorldStep } from "@destack/environment";
import { ObjectType, role } from "@destack/object";
import {
    DynamicObjectClient,
    type LiveQuery,
    LiveText,
    Observable,
    ObjectClient,
} from "@destack/object/client";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import type { PackageId } from "@destack/package";
import type { SetStep } from "@destack/package/declare";
import {
    Digest,
    type Identifier,
    type JsonObject,
    type JsonValue,
    present,
    schema,
} from "@destack/schema";
import stackPackage from "@destack/stack/package";
import { replica, Scope } from "@destack/sync";
import { type Counters, type Driver, onTestFinished } from "@destack/test";
import {
    type MachineStep,
    worldInteraction,
    type WorldObservation,
} from "../../benchmark/index.ts";
import { FILES_NODE, OWNER_EMAIL, TestComputer } from "./computer.ts";
import { TestDistribution } from "./distribution.ts";

/** The most simulated milliseconds a call waits for every device to see its result. */
const VISIBLE_MILLISECONDS = 30_000;

/** The fixture packages worlds install, which the run's setup builds before any world. */
export const WORLD_FIXTURES = ["apps", "board"] as const;

/** The fixture packages only scenarios install, which no template ships. */
export const SCENARIO_FIXTURES = ["mention"] as const;

/** The run's provided values the worlds read. */
declare module "vitest" {
    /** The values a run's setup provides to its tests. */
    export interface ProvidedContext {
        /** The manifest digests of the run's builds, by name, absent without a setup. */
        built?: Readonly<Record<string, string>>;
        /** The manifest digests of the packages only scenarios install, by name. */
        scenarios?: Readonly<Record<string, string>>;
        /** Where computers' state directories move after their tests, absent without a setup. */
        trash?: string;
    }
}

/** The name of the owner's home computer in observations. */
const HOME = "home";

/** Milliseconds per second. */
const MILLISECONDS_PER_SECOND = 1000;

/** The rows a live query of objects shows. */
const ROWS = schema.array(schema.record(schema.string(), schema.unknown()));

/** The placeholder a call's input strings take for the call's sequence number. */
const SEQUENCE = "{n}";

/** The module of a fixture package, exporting its handle with its object types. */
const FixturePackage = schema.looseObject({
    /** The package's handle. */
    default: schema.looseObject({
        /** The declared object types, by name. */
        objects: schema.record(schema.string(), schema.unknown()),
    }),
});

/** The output of a call creating or changing an object: its identifier and its last update. */
const CallOutput = schema.looseObject({
    /** The object's identifier. */
    id: schema.string(),
    /** When the object changed, absent for an object without timestamps. */
    updatedAt: schema.number().exactOptional(),
});

/** A member's device: an object client following the installed objects in its own replica. */
interface MemberDevice {
    /** The device's replica. */
    readonly database: DatabaseConnection;
    /** The client following the installed objects. */
    readonly client: ObjectClient;
    /** The rows each followed type's live query shows, as one watch has them, by type name. */
    readonly shown: ReadonlyMap<string, Observable<readonly Record<string, unknown>[]>>;
    /** Stops the client's loops. */
    readonly following: AbortController;
}

/** A member of the world: their login on the home computer and their devices. */
interface Member {
    /** The member's login on the home computer's device. */
    readonly login: Login["row"];
    /** The member's devices, in opening order. */
    readonly devices: MemberDevice[];
}

/** A driver that plays world scenarios and benchmarks over the owner's home and its members. */
export class WorldDriver implements Driver<typeof worldInteraction> {
    /** The interaction the driver speaks. */
    static readonly interaction = worldInteraction;

    /** The residency's cloud computer serving the owner's home. */
    readonly home: TestComputer;
    /** The owner's login on the home computer's device. */
    readonly login: Identifier<"login">;
    /** The computers that joined, by name, the home's among them. */
    readonly computers: Map<string, TestComputer>;
    /** The members, in joining order. */
    readonly members: Member[] = [];
    /** The installed package's object types by name, with the stack's role. */
    readonly #objects = new Map<string, { type: ObjectType; packageId: PackageId }>([
        [role.name, { type: role, packageId: stackPackage.id }],
    ]);
    /** The object of each type created last. */
    readonly #latest = new Map<string, string>();
    /** The parent of each object created in another, by identifier. */
    readonly #parents = new Map<string, string>();
    /** The members granted each shared object, by index, whose devices read it and its contents. */
    readonly #shared = new Map<string, Set<number>>();
    /** Each member's relationship granting a relation, by object type, object and member index. */
    readonly #relationships = new Map<string, string>();
    /** The client calling each object type's package, by type and caller. */
    readonly #callers = new Map<string, DynamicObjectClient>();
    /** The texts the devices follow, by device, object and field. */
    readonly #texts = new Map<string, LiveText>();
    /** The characters typed into each text so far, by object and field. */
    readonly #typed = new Map<string, number>();
    /** The calls taken so far, numbering each call's input. */
    #sequence = 0;
    /** The failures the devices' loops reported. */
    readonly #failures: unknown[] = [];

    /** Take a started world. */
    constructor(home: TestComputer, login: Identifier<"login">) {
        this.home = home;
        this.login = login;
        this.computers = new Map([[HOME, home]]);
    }

    /** The world's clock, moved by the world's loop. */
    get clock(): Clock {
        return this.home.clock;
    }

    /** Start the owner's home on its cloud computer. */
    static async start(): Promise<WorldDriver> {
        const { home, login } = await TestComputer.home();

        return new WorldDriver(home, login);
    }

    /** Close the followed texts, then the computers that joined, the home's last. */
    async [Symbol.asyncDispose](): Promise<void> {
        for (const text of this.#texts.values()) {
            await text.close();
        }
        for (const computer of [...this.computers.values()].toReversed()) {
            await computer[Symbol.asyncDispose]();
        }
    }

    /** Take one step, moving the world's time only while it runs. */
    async act(step: SetStep | WorldStep | MachineStep): Promise<void> {
        // read the world's simulation
        const { simulation } = this.home.world;

        // take a step every world shares in the world's simulation
        if (World.isStep(step)) {
            await simulation.take(step);
        }
        // start a personal computer with the owner signed in
        else if (step.action === "open") {
            const computer = await TestComputer.open({
                world: this.home.world,
                node: step.computer,
            });
            const login = await computer.signIn(OWNER_EMAIL);
            await computer.enroll(login, this.home.world.owner.account);
            this.computers.set(step.computer, computer);
        }
        // build a fixture package outside the world's time, then install it in the owner's home
        else if (step.action === "install") {
            const build = await WorldDriver.fixture(
                step.fixture,
                this.home.world.simulation.environment(FILES_NODE),
            );
            await this.#install(step.fixture, build);
        }
        // sign members in, each following the installed objects on their devices
        else if (step.action === "join") {
            await this.#join(step.members, step.devices, step.member);
        }
        // grant every member a relation on the object created last
        else if (step.action === "share") {
            await this.#share(step.object, step.relation);
        }
        // revoke a member's relation on the object created last
        else if (step.action === "unshare") {
            await this.#unshare(step.object, step.member);
        }
        // call an object method and wait until every device sees the result
        else if (step.action === "call") {
            await this.#call(step);
        }
        // send unstored events and wait until every reader's device has them
        else if (step.action === "broadcast") {
            await this.#broadcast(step);
        }
        // type text into a text field, members at once, and wait until every device shows it
        else if (step.action === "type") {
            await this.#type(step);
        }
        // refuse the set step, as a world has no example properties
        else {
            throw new TypeError("a world takes no example properties");
        }

        // end the step once the work it started settles at the current time, failing on a device's failure
        await simulation.clock.sleep(this.clock.now() - simulation.clock.now());
        await simulation.idle();
        if (this.#failures.length > 0) {
            throw new AggregateError(this.#failures.splice(0), "a member's device failed");
        }
    }

    /** Read one observation: the replicas a computer's machine and device databases follow. */
    async observe(observation: WorldObservation): Promise<JsonValue> {
        // count the replicas of the computer's machine and device databases
        const computer = present(
            this.computers.get(observation.computer),
            `a computer ${observation.computer} in the world`,
        );
        const databases = [computer.machine.database, computer.device.database];
        const rows = await Promise.all(
            databases.map((database) => database.select().from(replica)),
        );

        return rows.flat().length;
    }

    /** Read the fingerprint of everything the world did so far. */
    fingerprint(): number {
        return this.home.world.simulation.fingerprint;
    }

    /** Read the world's counters from its telemetry. */
    count(): Promise<Counters> {
        return this.home.world.simulation.telemetry.counts();
    }

    /** Open a fixture package's build in the run's store, which stays open until the test ends. */
    static async fixture(
        fixture: string,
        environment?: Environment<"files" | "process" | "host">,
    ): Promise<BuildReader> {
        // open the store for as long as the test reads the build
        const built = present(
            { ...TestDistribution.built(), ...TestDistribution.scenarios() }[fixture],
            `the run's build of ${fixture}`,
        );
        const store = await testStore(environment);
        onTestFinished(() => store[Symbol.asyncDispose]());

        return await store.contents(Digest.parse(built));
    }

    /** Build a fixture package's server outputs. */
    static build(fixture: string, compilers: CompilerPool): Promise<BuildReader> {
        return TestDistribution.compile(compilers, WorldDriver.#fixture(fixture));
    }

    /** Install a fixture build in the owner's home as the owner, recording its object types. */
    async #install(fixture: string, build: BuildReader): Promise<void> {
        // install the build as the owner
        const { world } = this.home;
        await this.home.install(this.login, world.owner.home, build, fixture);

        // record its object types by name
        const packageId = PackageManifest.package(build.manifest).id;
        const module: unknown = await world.simulation.scheduler.track(
            () => import(`${WorldDriver.#fixture(fixture)}src/package.ts`),
            "host",
        );
        const handle = FixturePackage.parse(module).default;
        for (const type of Object.values(handle.objects)) {
            if (ObjectType.is(type)) {
                this.#objects.set(type.name, { type, packageId });
            }
        }
    }

    /** Find a fixture package's directory below the machine's tests. */
    static #fixture(fixture: string): string {
        return fileURLToPath(new URL(`../../../tests/fixture/${fixture}/`, import.meta.url));
    }

    /** Sign members in following the installed objects, or open a member's further devices. */
    async #join(members: number, devices: number, member: number | undefined): Promise<void> {
        // open more devices of an existing member
        if (member !== undefined) {
            const joined = present(this.members[member], `a member ${member}`);
            for (let device = 0; device < devices; device += 1) {
                joined.devices.push(await this.#open(joined.login));
            }

            return;
        }

        // sign each new member in on their devices
        for (let index = 0; index < members; index += 1) {
            const login = await this.home.signIn(`member-${this.members.length}@bench.test`);
            const opened: MemberDevice[] = [];
            for (let device = 0; device < devices; device += 1) {
                opened.push(await this.#open(login));
            }
            this.members.push({ login, devices: opened });
        }
    }

    /** Open a member's device: a client following every installed object in its own replica. */
    async #open(login: Login["row"]): Promise<MemberDevice> {
        // follow the installed objects at their package in the owner's home, as the member
        const { device, world } = this.home;
        const installed = [...this.#objects.values()].filter(
            ({ packageId }) => packageId !== stackPackage.id,
        );
        const packageId = present(installed[0], "an installed package").packageId;
        const endpoint = (await device.session(login.id)).endpoint(
            packageId,
            await device.locate(world.owner.home),
        );
        const objects = Object.fromEntries(installed.map(({ type }) => [type.name, type]));
        const database = await connectBunSqlite(":memory:", ObjectClient.tables(objects), {
            environment: device.environment,
        });
        const client = await ObjectClient.open({
            environment: device.environment,
            database,
            objects,
            scope: world.owner.home,
            caller: principal.user.reference(Scope.universe.id, login.subjectId),
            endpoint,
            reconnect: () => endpoint,
        });

        // follow every object type and run the client's loops until the world ends
        const queries = new Map(
            Object.values(objects).flatMap((type) => {
                const live = client.query[type.name]?.findMany({}).subscribe();

                return live === undefined ? [] : [[type.name, live] as const];
            }),
        );
        const following = new AbortController();
        const report = (error: unknown) => this.#failures.push(error);
        void client.follow(following.signal, report);
        void client.push(following.signal, report);

        // end once the replica has every followed query, with each one's rows as one watch shows them
        await Promise.all([...queries.values()].map((live) => live.ready));
        const shown = new Map(
            [...queries].map(([name, live]) => [name, WorldDriver.#show(live, following.signal)]),
        );

        return { database, client, shown, following };
    }

    /** Grant every member a relation on the object of a type created last, as the owner. */
    async #share(object: string, relation: string): Promise<void> {
        // grant each member the relation as the owner, recording each relationship, then wait until their devices see the object
        const id = present(this.#latest.get(object), `a created ${object}`);
        const client = await this.#caller(object, undefined);
        for (const [index, member] of this.members.entries()) {
            const subject = principal.user.reference(Scope.universe.id, member.login.subjectId);
            const granted = CallOutput.parse(
                await client.call(object, "grant", { id, relation, subject }),
            );
            this.#relationships.set(`${object} ${id} ${index}`, granted.id);
        }
        this.#shared.set(id, new Set(this.members.keys()));
        await this.#visible(object, id, undefined);
    }

    /** Revoke a member's relationship on a type's latest object, until their devices lose it. */
    async #unshare(object: string, member: number): Promise<void> {
        // revoke the member's relationship as the owner
        const id = present(this.#latest.get(object), `a created ${object}`);
        const relationshipId = present(
            this.#relationships.get(`${object} ${id} ${member}`),
            `member ${member}'s relationship on ${object} ${id}`,
        );
        const client = await this.#caller(object, undefined);
        await client.call(object, "revoke", { id, relationshipId });
        this.#shared.get(id)?.delete(member);

        // wait at each of the member's devices' live queries until the row is gone
        const deadline = this.clock.timeout(VISIBLE_MILLISECONDS);
        const devices = present(this.members[member], `a member ${member}`).devices;
        const gone = await Promise.all(
            devices.map((device) =>
                WorldDriver.#shown(device, object, deadline, (rows) =>
                    rows.every((row) => row["id"] !== id),
                ),
            ),
        );
        if (gone.includes(false)) {
            throw new TypeError(`${object} ${id} stayed on member ${member}'s devices`, {
                cause: [this.home.world.telemetry.describe(), ...this.#failures],
            });
        }
    }

    /** Call an object method, repeated at a rate, then wait until every reader's device sees it. */
    async #call(step: Extract<MachineStep, { readonly action: "call" }>): Promise<void> {
        // start each call on time, without waiting for the one before, as arrivals come on
        const call = await this.#calling(step.object, step.method, step.member);
        const calls: Promise<unknown>[] = [];
        for (let index = 0; index < (step.repeat ?? 1); index += 1) {
            if (index > 0 && step.rate !== undefined) {
                await this.clock.sleep(MILLISECONDS_PER_SECOND / step.rate);
            }
            calls.push(call(this.#input(step)));
        }

        // record the last created or changed object as its type's latest, and wait until every device sees it
        const outputs = await Promise.all(calls);
        const output = CallOutput.parse(present(outputs.at(-1), "a call's output"));
        this.#latest.set(step.object, output.id);
        if (step.in !== undefined) {
            this.#parents.set(
                output.id,
                present(this.#latest.get(step.in), `a created ${step.in}`),
            );
        }
        await this.#visible(step.object, output.id, output.updatedAt);
    }

    /** Call a method as the owner or a member, an ephemeral type's from their first device. */
    async #calling(
        object: string,
        method: string,
        member: number | undefined,
    ): Promise<(input: JsonObject) => Promise<unknown>> {
        // write an ephemeral type's rows as a device's client, which streams as their writer
        const { type } = present(this.#objects.get(object), `an object type ${object}`);
        if (type.storage === "ephemeral") {
            const writer = present(
                this.members[present(member, `the member writing ${object}`)]?.devices[0],
                `member ${member}'s first device`,
            );

            return (input) => writer.client.call(type, method, input).predicted;
        }

        // call other types through the package's client
        const client = await this.#caller(object, member);

        return (input) => client.call(object, method, input);
    }

    /** Send unstored events on a type's latest object at a rate until every reader has them. */
    async #broadcast(step: Extract<MachineStep, { readonly action: "broadcast" }>): Promise<void> {
        // listen on every reader's device before the first event
        const { type } = present(this.#objects.get(step.object), `an object type ${step.object}`);
        const id = present(this.#latest.get(step.object), `a created ${step.object}`);
        const deadline = this.clock.timeout(VISIBLE_MILLISECONDS);
        const devices = this.#readers(id).flatMap((member) => member.devices);
        const received = devices.map(async ({ client }) => {
            // take each event until all arrived, failing once the stream ends first
            const events = client.broadcasts(type, id, deadline)[Symbol.asyncIterator]();
            for (let count = 0; count < step.repeat; count += 1) {
                if ((await events.next()).done === true) {
                    return false;
                }
            }
            await events.return?.(undefined);

            return true;
        });

        // send the events on time from the member's first device
        const sender = present(
            this.members[step.member]?.devices[0],
            `member ${step.member}'s first device`,
        );
        for (let index = 0; index < step.repeat; index += 1) {
            if (index > 0) {
                await this.clock.sleep(MILLISECONDS_PER_SECOND / step.rate);
            }
            this.#sequence += 1;
            const event = schema
                .jsonObject()
                .parse(
                    JSON.parse(
                        JSON.stringify(step.event)
                            .replaceAll(`"${SEQUENCE}"`, String(this.#sequence))
                            .replaceAll(SEQUENCE, String(this.#sequence)),
                    ),
                );
            await sender.client.broadcast(type, id, event);
        }

        // require every reader's device to have every event
        if ((await Promise.all(received)).includes(false)) {
            throw new TypeError(`${step.object} ${id} events did not reach every device`, {
                cause: [this.home.world.telemetry.describe(), ...this.#failures],
            });
        }
    }

    /** Fill a call's input: its parent, its target and its sequence number. */
    #input(step: Extract<MachineStep, { readonly action: "call" }>): JsonObject {
        // replace a whole `{n}` string by the number and other `{n}` by its digits
        this.#sequence += 1;
        const sequence = String(this.#sequence);
        const text = JSON.stringify(step.input)
            .replaceAll(`"${SEQUENCE}"`, sequence)
            .replaceAll(SEQUENCE, sequence);

        return {
            ...schema.jsonObject().parse(JSON.parse(text)),
            ...(step.in === undefined ? {} : this.#parent(step.object, step.in)),
            ...(step.on === undefined
                ? {}
                : { id: present(this.#latest.get(step.on), `a created ${step.on}`) }),
        };
    }

    /** Name a created object's parent: by reference for an attached type, else by identifier. */
    #parent(object: string, parent: string): JsonObject {
        // read the parent created last and the created type
        const id = present(this.#latest.get(parent), `a created ${parent}`);
        const { type } = present(this.#objects.get(object), `an object type ${object}`);
        const owner = present(this.#objects.get(parent), `an object type ${parent}`);

        // reference an attachment's parent by package, type and identifier
        return type.parent?.object === "any"
            ? { parent: { packageId: owner.packageId, type: parent, id } }
            : { parentId: id };
    }

    /** Type into a text field, members at once, until every reader's device shows all of it. */
    async #type(step: Extract<MachineStep, { readonly action: "type" }>): Promise<void> {
        // type each member's characters at the field's end from their first device, members at once
        const id = present(this.#latest.get(step.object), `a created ${step.object}`);
        const typing = this.members.slice(0, step.members);
        if (typing.length < step.members) {
            throw new TypeError(`${step.members} members type, but ${typing.length} joined`);
        }
        await Promise.all(
            typing.map(async (member) => {
                const device = present(member.devices[0], "a member's device");
                const text = this.#text(device, step.object, id, step.field);
                for (const character of step.text) {
                    const length = (await text.read()).text().length;
                    await text.change({ from: length, to: length, insert: character });
                }
            }),
        );

        // wait until every device's text has every character typed so far
        const key = `${step.object} ${id} ${step.field}`;
        const typed = (this.#typed.get(key) ?? 0) + step.members * step.text.length;
        this.#typed.set(key, typed);
        const deadline = this.clock.timeout(VISIBLE_MILLISECONDS);
        const devices = this.#readers(id).flatMap((member) => member.devices);
        const shown = await Promise.all(
            devices.map(async (device) => {
                const text = this.#text(device, step.object, id, step.field);
                for await (const sequence of text.watch(deadline)) {
                    if (sequence.text().length >= typed) {
                        return true;
                    }
                }

                return false;
            }),
        );
        if (shown.includes(false)) {
            throw new TypeError(`${step.object} ${id} ${step.field} did not reach every device`, {
                cause: [this.home.world.telemetry.describe(), ...this.#failures],
            });
        }
    }

    /** List the members who read an object: those granted it or the object it is in. */
    #readers(id: string): Member[] {
        // walk up to the nearest shared object
        let current: string | undefined = id;
        while (current !== undefined && !this.#shared.has(current)) {
            current = this.#parents.get(current);
        }
        const shared =
            current === undefined ? new Set<number>() : (this.#shared.get(current) ?? new Set());

        return this.members.filter((_member, index) => shared.has(index));
    }

    /** Follow a text field on a device once. */
    #text(device: MemberDevice, object: string, id: string, field: string): LiveText {
        // open the device's text once, keyed by its replica
        const index = this.members.flatMap((member) => member.devices).indexOf(device);
        const key = `${index} ${object} ${id} ${field}`;
        const known = this.#texts.get(key);
        if (known !== undefined) {
            return known;
        }
        const { type } = present(this.#objects.get(object), `an object type ${object}`);
        const opened = LiveText.open(device.client, type, id, field);
        this.#texts.set(key, opened);

        return opened;
    }

    /** Wait until every member device's replica has an object at least as recent as written. */
    async #visible(object: string, id: string, updatedAt: number | undefined): Promise<void> {
        // leave objects no device follows, such as roles
        const followed = this.#objects.get(object);
        if (followed === undefined || followed.packageId === stackPackage.id) {
            return;
        }

        // wait at each device's live query for the row at the written revision
        const deadline = this.clock.timeout(VISIBLE_MILLISECONDS);
        const devices = this.#readers(id).flatMap((member) => member.devices);
        const seen = await Promise.all(
            devices.map((device) =>
                WorldDriver.#shown(device, object, deadline, (rows) =>
                    rows.some(
                        (row) =>
                            row["id"] === id &&
                            (updatedAt === undefined ||
                                (typeof row["updatedAt"] === "number" &&
                                    row["updatedAt"] >= updatedAt)),
                    ),
                ),
            ),
        );
        if (seen.includes(false)) {
            throw new TypeError(`${object} ${id} did not reach every device`, {
                cause: [this.home.world.telemetry.describe(), ...this.#failures],
            });
        }
    }

    /** Collect the rows a live query shows, as its one watch yields them until stopped. */
    static #show(
        live: LiveQuery<unknown>,
        signal: AbortSignal,
    ): Observable<readonly Record<string, unknown>[]> {
        const shown = new Observable<readonly Record<string, unknown>[]>([]);
        void (async () => {
            for await (const rows of live.watch(signal)) {
                shown.set(ROWS.parse(rows));
            }
        })();

        return shown;
    }

    /** Wait until a device shows rows of a type a check accepts, false at the deadline. */
    static async #shown(
        device: MemberDevice,
        object: string,
        deadline: AbortSignal,
        isShown: (rows: readonly Record<string, unknown>[]) => boolean,
    ): Promise<boolean> {
        const shown = present(device.shown.get(object), `the shown rows of ${object}`);
        for await (const rows of shown.watch(deadline)) {
            if (isShown(rows)) {
                return true;
            }
        }

        return false;
    }

    /** Call an object's package as the owner or a member through the home computer's device. */
    async #caller(object: string, member: number | undefined): Promise<DynamicObjectClient> {
        // reuse one client per object type and caller, as an app reuses its client
        const key = `${object} ${member ?? "owner"}`;
        const known = this.#callers.get(key);
        if (known !== undefined) {
            return known;
        }

        // sign as the owner or the member, at the object's package in the owner's home
        const { device, world } = this.home;
        const { type, packageId } = present(this.#objects.get(object), `an object type ${object}`);
        const login =
            member === undefined
                ? this.login
                : present(this.members[member], `a member ${member}`).login.id;
        const client = new DynamicObjectClient({
            objects: [type],
            scope: world.owner.home,
            endpoint: (await device.session(login)).endpoint(
                packageId,
                await device.locate(world.owner.home),
            ),
            environment: device.environment,
        });
        this.#callers.set(key, client);

        return client;
    }
}
