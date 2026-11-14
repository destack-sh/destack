import { principal, Role, StepUp } from "@destack/access";
import { createClient } from "@destack/object/client";
import { space } from "@destack/account/object";
import type { Login } from "@destack/account/native";
import type { Workspace } from "@destack/build";
import type { DatabaseConnection } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { type Clock, type Environment, World } from "@destack/environment";
import { ObjectType, PackageMount, role, SpaceMount } from "@destack/object";
import { DynamicObjectClient, LiveText, ObjectClient } from "@destack/object/client";
import { PackageManifest } from "@destack/package/manifest";
import type { PackageId } from "@destack/package";
import type { Given, SetStep } from "@destack/package/declare";
import { type Identifier, type JsonObject, type JsonValue, present, schema } from "@destack/schema";
import stackPackage from "@destack/stack/package";
import { RequestId, Scope } from "@destack/sync";
import type { SimulatedTelemetry } from "@destack/environment/simulation";
import type { Driver } from "@destack/test";
import {
    type MachineAction,
    type MachineCall,
    type MachineStep,
    machineInteraction,
    type MachineObservation,
} from "../scenario/interaction.ts";
import { ObjectStep } from "@destack/object/test";
import { FILES_NODE, OWNER_EMAIL, TestComputer } from "../computer/test/computer.ts";
import { TestDistribution } from "../computer/test/distribution.ts";
import { TestFixture } from "../computer/test/fixture.ts";
import { Launcher, type LauncherMachine } from "../launcher/index.ts";

/** The most simulated milliseconds a call waits for every device to see its result. */
const VISIBLE_MILLISECONDS = 30_000;

/** The name of the owner's home computer in observations. */
const HOME = "home";

/** A member's name, such as `member-0`, holding their joining index. */
const MEMBER = /^member-(\d+)$/u;

/** The owner's name among a world's people. */
const OWNER = "owner";

/** The second factor moving a space asks of its owner: assurance two within fifteen minutes. */
const MOVE_STEP_UP = StepUp.schema.parse({ assurance: 2, maxAge: 900_000 });

/** The status a launch's token trade answers with, See Other to the page (RFC 9110). */
const REDIRECT_STATUS = 303;

/** Milliseconds per second. */
const MILLISECONDS_PER_SECOND = 1000;

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
    /** The device's host in the world, such as `member-0.device-1`. */
    readonly host: string;
    /** The device's replica. */
    readonly database: DatabaseConnection;
    /** The client following the installed objects. */
    readonly client: ObjectClient;
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
export class MachineDriver implements Driver<MachineStep, MachineObservation> {
    /** The interaction the driver speaks. */
    static readonly interaction = machineInteraction;
    /** The settings of the worlds: SQLite in WAL with synchronous NORMAL, as in production. */
    static readonly configuration = { durability: "normal" };

    /** The residency's cloud computer serving the owner's home, a new one after each restart. */
    home: TestComputer;
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
    /** The identifiers each call bound, by name, in call order. */
    readonly #bound = new Map<string, string[]>();
    /** The client calling each object type's package, by type and person. */
    readonly #callers = new Map<string, DynamicObjectClient>();
    /** The texts the devices follow, by device, object and field. */
    readonly #texts = new Map<string, LiveText>();
    /** The characters typed into each text so far, by object and field. */
    readonly #typed = new Map<string, number>();
    /** The invitation each member was sent last, by member. */
    readonly #invitations = new Map<string, string>();
    /** The spaces the world created, by name. */
    readonly #spaces = new Map<string, Identifier<"space">>();
    /** The relay personal computers reach each other through, opened with the first of them. */
    #relay: Awaited<ReturnType<typeof TestComputer.relay>> | undefined;
    /** The failures the devices' loops reported. */
    readonly #failures: unknown[] = [];

    /** Take a started world. */
    constructor(home: TestComputer, signedIn: Identifier<"login">) {
        this.home = home;
        this.login = signedIn;
        this.computers = new Map([[HOME, home]]);
    }

    /** The world's clock, moved by the world's loop. */
    get clock(): Clock {
        return this.home.clock;
    }

    /** Start the owner's home on its cloud computer, given no examples. */
    static async start(_given: Given): Promise<MachineDriver> {
        const { home, login: signedIn } = await TestComputer.home({ node: HOME });

        return new MachineDriver(home, signedIn);
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
    async act(step: SetStep | MachineStep): Promise<void> {
        // read the world's simulation
        const { simulation } = this.home.world;

        // take a step every world shares in the world's simulation
        if (World.isStep(step)) {
            await simulation.take(step);
        }
        // call an object's method until every device reflects the home
        else if ("call" in step) {
            await this.#call(step);
        }
        // start a personal computer through the relay with the owner signed in
        else if (step.action === "open") {
            this.#relay ??= await TestComputer.relay(this.home.world);
            const computer = await TestComputer.open({
                world: this.home.world,
                node: step.computer,
                relay: this.#relay.url,
            });
            const signedIn = await computer.signIn(OWNER_EMAIL);
            await computer.enroll(signedIn, this.home.world.owner.account);
            this.computers.set(step.computer, computer);
        }
        // sign a person in, signing them up the first time
        else if (step.action === "sign-in") {
            await this.home.signIn(step.email);
        }
        // publish a fixture package as the platform's operator
        else if (step.action === "publish") {
            await this.home.publish(TestDistribution.lockOf(step.fixture));
        }
        // install a published fixture package in the owner's home
        else if (step.action === "install") {
            await this.#install(step.fixture);
        }
        // update an installation to a published release
        else if (step.action === "update") {
            const { world } = this.home;
            const build = await TestDistribution.fixture(
                step.release,
                world.simulation.environment(FILES_NODE),
            );
            await this.home.update(this.login, world.owner.home, step.installation, build);
        }
        // move a created space to a computer until it serves the space
        else if (step.action === "move") {
            await this.#move(step.space, step.computer);
        }
        // launch a view as the owner until its document loads
        else if (step.action === "launch") {
            await this.#launch(step.installation, step.view);
        }
        // restart the home's computer until it serves the home again
        else if (step.action === "restart") {
            await this.#restart();
        }
        // invite a member to own a created space, as the owner
        else if (step.action === "invite") {
            await this.#invite(step.member, step.space);
        }
        // accept a member's latest invitation as them
        else if (step.action === "accept") {
            await this.#accept(step.member);
        }
        // sign members in, each following the installed objects on their devices
        else if (step.action === "join") {
            await this.#join(step.members, step.devices);
        }
        // connect more devices of a joined member
        else if (step.action === "connect") {
            await this.#connect(step.member, step.devices);
        }
        // create a space in the owner's account until the machine serves it
        else if (step.action === "create") {
            const { account } = this.home.world.owner;
            this.#spaces.set(step.space, await this.home.space(this.login, account, step.space));
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
        await simulation.run(this.clock.now());
        if (this.#failures.length > 0) {
            throw new AggregateError(this.#failures.splice(0), "a member's device failed");
        }
    }

    /** Read one observation: the replicas a computer's machine and device databases follow. */
    async observe(observation: MachineObservation): Promise<JsonValue> {
        // read the type's rows in the owner's home, or in a member device's replica
        const { type } = present(
            this.#objects.get(observation.count),
            `an object type ${observation.count}`,
        );
        const database =
            observation.on === undefined
                ? (await this.home.machine.space(this.home.world.owner.home)).database
                : present(
                      this.members
                          .flatMap((member) => member.devices)
                          .find((device) => device.host === observation.on),
                      `a member device ${observation.on}`,
                  ).database;

        return (await database.select().from(type.table)).length;
    }

    /** Read the fingerprint of everything the world did so far. */
    fingerprint(): number {
        return this.home.world.simulation.fingerprint;
    }

    /** The telemetry the world records. */
    get telemetry(): SimulatedTelemetry {
        return this.home.world.simulation.telemetry;
    }

    /** Build and lock a fixture package at its authored version. */
    static build(
        fixture: string,
        workspace: Workspace,
    ): ReturnType<typeof TestDistribution.compile> {
        return TestDistribution.compile(workspace, new TestFixture(fixture).directory);
    }

    /** Install a published fixture in the owner's home as the owner, recording its object types. */
    async #install(fixture: string): Promise<void> {
        // open the build outside the world's time, and install it as the owner
        const { world } = this.home;
        const build = await TestDistribution.fixture(
            fixture,
            world.simulation.environment(FILES_NODE),
        );
        await this.home.install(this.login, world.owner.home, build, fixture);

        // record its object types by name
        const packageId = PackageManifest.package(build.manifest).id;
        const module: unknown = await world.simulation.scheduler.run(
            () => import(`${new TestFixture(fixture).directory}src/package.ts`),
        );
        const handle = FixturePackage.parse(module).default;
        for (const type of Object.values(handle.objects)) {
            if (ObjectType.is(type)) {
                this.#objects.set(type.name, { type, packageId });
            }
        }
    }

    /** Sign new members in, each on devices following the installed objects. */
    async #join(members: number, devices: number): Promise<void> {
        // sign each new member in on their devices
        for (let index = 0; index < members; index += 1) {
            const name = `member-${this.members.length}`;
            const signedIn = await this.home.signIn(`${name}@scenario.test`);
            const opened: MemberDevice[] = [];
            for (let device = 0; device < devices; device += 1) {
                opened.push(await this.#open(signedIn, `${name}.device-${device}`));
            }
            this.members.push({ login: signedIn, devices: opened });
        }
    }

    /** Connect more devices of a joined member. */
    async #connect(member: string, devices: number): Promise<void> {
        // open each device after the member's last
        const joined = this.#member(member);
        for (let device = 0; device < devices; device += 1) {
            joined.devices.push(
                await this.#open(joined.login, `${member}.device-${joined.devices.length}`),
            );
        }
    }

    /** Open a member's device: a client following every installed object in its own replica. */
    async #open(signedIn: Login["row"], node: string): Promise<MemberDevice> {
        // follow the installed objects at their package in the owner's home, as the member from the device's own host
        const { world } = this.home;
        const environment = world.simulation.environment(node);
        const installed = [...this.#objects.values()].filter(
            ({ packageId }) => packageId !== stackPackage.id,
        );
        const packages = [...new Set(installed.map(({ packageId }) => packageId))];
        const [packageId] = packages;
        if (packageId === undefined || packages.length > 1) {
            throw new TypeError(
                `devices follow one installed package's objects, not ${packages.length}`,
            );
        }
        const launcher = await this.#launcher(signedIn.id, environment);
        const endpoint = launcher.endpoint(
            packageId,
            await launcher.machine.locate(world.owner.home),
        );
        const objects = Object.fromEntries(installed.map(({ type }) => [type.name, type]));
        const database = await connectBunSqlite(":memory:", ObjectClient.tables(objects), {
            environment,
        });
        const client = await ObjectClient.open({
            environment,
            database,
            objects,
            scope: world.owner.home,
            caller: principal.user.reference(Scope.universe.id, signedIn.subjectId),
            endpoint,
            reconnect: () => endpoint,
        });

        // follow every object type and run the client's loops until the world ends
        const queries = new Map(
            Object.values(objects).map((type) => {
                const query = present(client.query[type.name], `the query of ${type.name}`);

                return [type.name, query.findMany({}).subscribe()] as const;
            }),
        );
        const following = new AbortController();
        const report = (error: unknown) => this.#failures.push(error);
        void client.follow(following.signal, report);
        void client.push(following.signal, report);

        // end once the replica has every followed query
        await Promise.all([...queries.values()].map((live) => live.ready));

        return { host: node, database, client, following };
    }

    /** Call a method as a person, a burst at a rate, binding its result, until settled. */
    async #call(step: MachineCall): Promise<void> {
        // resolve the input, then start each call on time without waiting for the one before
        const { type: object, method } = ObjectStep.target(step);
        const call = await this.#calling(object, method, step.as);
        const input = ObjectStep.input(step.input, this.#bound, (person) =>
            this.#principal(person),
        );
        const calls: Promise<unknown>[] = [];
        for (let index = 0; index < (step.repeat ?? 1); index += 1) {
            if (index > 0 && step.rate !== undefined) {
                await this.home.world.simulation.sleep(MILLISECONDS_PER_SECOND / step.rate);
            }
            calls.push(call(input));
        }

        // bind the object the last call returns, and settle every device
        const outputs = await Promise.all(calls);
        if (step.bind !== undefined) {
            const id = CallOutput.parse(present(outputs.at(-1), "a call's output")).id;
            this.#bound.set(step.bind, [...(this.#bound.get(step.bind) ?? []), id]);
        }
        await this.#settle();
    }

    /** Call a method as a person, an ephemeral type's from their first device. */
    async #calling(
        object: string,
        method: string,
        person: string,
    ): Promise<(input: JsonObject) => Promise<unknown>> {
        // write an ephemeral type's rows as the person's device's client, which streams as their writer
        const { type } = present(this.#objects.get(object), `an object type ${object}`);
        if (type.storage === "ephemeral") {
            const writer = present(this.#member(person).devices[0], `${person}'s first device`);

            return (input) => writer.client.call(type, method, input).predicted;
        }

        // call other types through the package's client
        const client = await this.#caller(object, person);

        return (input) => client.call(object, method, input);
    }

    /** Wait until every member device's replica reflects the owner's home up to its last commit. */
    async #settle(): Promise<void> {
        // read the home's position, then wait at every device for it
        const { simulation, owner } = this.home.world;
        const { database } = await this.home.machine.space(owner.home);
        const position = await database.log.position();
        const deadline = this.clock.timeout(VISIBLE_MILLISECONDS);
        const devices = this.members.flatMap((member) => member.devices);
        const reached = await simulation.wait(
            Promise.all(
                devices.map(({ client }) =>
                    client.replica.wait(client.database, position, deadline),
                ),
            ),
        );
        if (reached.includes(false)) {
            throw new TypeError("a member's device did not reflect the home", {
                cause: [this.home.world.telemetry.describe(), ...this.#failures],
            });
        }
    }

    /** Send unstored events on an object from a member's device until its holders have them. */
    async #broadcast(
        step: Extract<MachineAction, { readonly action: "broadcast" }>,
    ): Promise<void> {
        // listen on every device holding the object before the first event
        const { type } = present(this.#objects.get(step.object), `an object type ${step.object}`);
        const id = this.#resolve(step.on);
        const deadline = this.clock.timeout(VISIBLE_MILLISECONDS);
        const received = (await this.#holders(type, id)).map(async ({ client }) => {
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
        const sender = present(this.#member(step.as).devices[0], `${step.as}'s first device`);
        for (let index = 0; index < step.repeat; index += 1) {
            if (index > 0) {
                await this.home.world.simulation.sleep(MILLISECONDS_PER_SECOND / step.rate);
            }
            await sender.client.broadcast(type, id, step.event);
        }

        // require every holder to have every event
        if ((await Promise.all(received)).includes(false)) {
            throw new TypeError(`${step.object} '${id}' events did not reach every device`, {
                cause: [this.home.world.telemetry.describe(), ...this.#failures],
            });
        }
    }

    /** Type into an object's text field, members at once, until every holder's device shows it. */
    async #type(step: Extract<MachineAction, { readonly action: "type" }>): Promise<void> {
        // type each member's characters at the field's end from their first device, members at once
        const { type } = present(this.#objects.get(step.object), `an object type ${step.object}`);
        const id = this.#resolve(step.on);
        const typing = this.members.slice(0, step.members);
        if (typing.length < step.members) {
            throw new TypeError(`'${step.members}' members type, but ${typing.length} joined`);
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

        // wait until every holder's text has every character typed so far
        const key = `${step.object} ${id} ${step.field}`;
        const typed = (this.#typed.get(key) ?? 0) + step.members * step.text.length;
        this.#typed.set(key, typed);
        const deadline = this.clock.timeout(VISIBLE_MILLISECONDS);
        const shown = await Promise.all(
            (await this.#holders(type, id)).map(async (device) => {
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
            throw new TypeError(
                `field '${step.field}' of ${step.object} '${id}' did not reach every device`,
                {
                    cause: [this.home.world.telemetry.describe(), ...this.#failures],
                },
            );
        }
    }

    /** List the member devices whose replicas hold an object. */
    async #holders(type: ObjectType, id: string): Promise<MemberDevice[]> {
        const devices = this.members.flatMap((member) => member.devices);
        const held = await Promise.all(
            devices.map(async (device) =>
                (await device.database.select().from(type.table)).some((row) => row["id"] === id),
            ),
        );

        return devices.filter((_device, index) => held[index] === true);
    }

    /** Resolve a reference to a bound object to its identifier. */
    #resolve(reference: string): string {
        const { id } = ObjectStep.input({ id: reference }, this.#bound, (person) =>
            this.#principal(person),
        );

        return schema.string().parse(id);
    }

    /** Read a member by name, such as `member-0`. */
    #member(person: string): Member {
        // refuse anyone but a member, such as the owner, who has no member devices
        const index = MEMBER.exec(person)?.[1];
        if (index === undefined) {
            throw new TypeError(`'${person}' is no member: only members have devices`);
        }

        return present(this.members[Number(index)], `a member ${person}`);
    }

    /** Read a person's login on the home computer: the owner's, or a member's. */
    #login(person: string): Identifier<"login"> {
        return person === OWNER ? this.login : this.#member(person).login.id;
    }

    /** Read a person's principal: the owner's, or a member's. */
    #principal(person: string): JsonValue {
        const subjectId =
            person === OWNER ? this.home.world.owner.id : this.#member(person).login.subjectId;

        return principal.user.reference(Scope.universe.id, subjectId);
    }

    /** Read the client calling a type's package as a person, one per type and person. */
    async #caller(object: string, person: string): Promise<DynamicObjectClient> {
        // reuse the client
        const key = `${object} ${person}`;
        const known = this.#callers.get(key);
        if (known !== undefined) {
            return known;
        }

        // sign in as the person, at the object's package in the owner's home
        const { device, world } = this.home;
        const { type, packageId } = present(this.#objects.get(object), `an object type ${object}`);
        const client = new DynamicObjectClient({
            objects: [type],
            scope: world.owner.home,
            endpoint: (await device.launcher(this.#login(person))).endpoint(
                packageId,
                await device.locate(world.owner.home),
            ),
            environment: device.environment,
        });
        this.#callers.set(key, client);

        return client;
    }

    /** Launch a view of the owner's home as the owner, then load it with its traded session. */
    async #launch(installation: string, view: string): Promise<void> {
        // launch the view and trade the launch's token for the session cookie
        const { computer, device, world } = this.home;
        const launcher = await device.launcher(this.login);
        const address = `home--${world.owner.handle}`;
        const launched = await launcher.launch({ space: address, installation, view, path: "/" });
        const traded = await computer.fetch(new Request(launched.url));
        const location = traded.headers.get("location");
        const cookie = traded.headers.get("set-cookie")?.split(";")[0];
        if (traded.status !== REDIRECT_STATUS || location === null || cookie === undefined) {
            throw new TypeError(`launching '${installation}/${view}' answered ${traded.status}`);
        }

        // load the document with the session, as the browser does
        const headers = { cookie, "sec-fetch-site": "same-origin" };
        const page = await computer.fetch(
            new Request(new URL(location, launched.url).href, { headers }),
        );
        const body = await page.text();
        if (!page.ok) {
            throw new TypeError(`'${installation}/${view}' answered ${page.status}: '${body}'`);
        }
    }

    /** Move a created space to a computer as its stepped-up owner, until the computer serves it. */
    async #move(name: string, computerName: string): Promise<void> {
        // step the owner up, and call the space's stack package with a token proving it
        const spaceId = present(this.#spaces.get(name), `a created space ${name}`);
        const target = present(this.computers.get(computerName), `a computer ${computerName}`);
        const { device, endpoint, environment } = this.home;
        const elevated = await this.home.elevate(MOVE_STEP_UP);
        const { user } = await device.launcher(elevated.id);
        const token = await user.token(stackPackage.id, { partition: spaceId }, MOVE_STEP_UP);
        const stack = createClient(
            stackPackage,
            {
                url: `${SpaceMount.url(endpoint, spaceId)}${PackageMount.path(stackPackage.id)}`,
                headers: { authorization: `Bearer ${token}` },
            },
            device.environment,
        );

        // move it, until the target computer serves it
        await stack.transfer.create({
            spaceId,
            requestId: RequestId.create(environment),
            targetId: target.machine.id,
        });
        await target.ready(spaceId);
    }

    /** Restart the home's computer, keeping its state, until it serves the owner's home again. */
    async #restart(): Promise<void> {
        // replace the computer, and drop the clients calling the stopped one
        this.home = await this.home.restart();
        this.computers.set(HOME, this.home);
        this.#callers.clear();

        // wait until it serves the home
        await this.home.ready(this.home.world.owner.home);
    }

    /** Invite a member to own a created space, as the owner, recording the invitation. */
    async #invite(member: string, name: string): Promise<void> {
        // invite the member's person to the space with the owner role
        const { world } = this.home;
        const invited = this.#member(member).login;
        const spaceId = present(this.#spaces.get(name), `a created space ${name}`);
        const session = (await this.home.device.launcher(this.login)).session;
        const invitation = await session.objects().invitation.create({
            scope: world.owner.account,
            requestId: RequestId.create(this.home.environment),
            object: space.reference(world.owner.account, spaceId),
            relationship: {
                role: Role.OWNER,
                subject: principal.user.reference(Scope.universe.id, invited.subjectId),
            },
        });
        this.#invitations.set(member, invitation.id);
    }

    /** Accept a member's latest invitation as them. */
    async #accept(member: string): Promise<void> {
        // accept as the member through their session
        const { world } = this.home;
        const signedIn = this.#member(member).login;
        const session = (await this.home.device.launcher(signedIn.id)).session;
        await session.objects().invitation.accept({
            scope: world.owner.account,
            requestId: RequestId.create(this.home.environment),
            id: present(this.#invitations.get(member), `member ${member}'s invitation`),
        });
    }

    /** Launch for a person from a device's own host, reaching the home at its public origin. */
    async #launcher(loginId: Identifier<"login">, environment: Environment): Promise<Launcher> {
        // prove the person's sign-in on the home computer, and locate spaces through its directory as it runs now
        const proven = await this.home.device.signedIn(loginId);
        const machine: LauncherMachine = {
            id: this.home.machine.id,
            environment,
            locate: async (address) => {
                const place = await this.home.device.locate(address);
                const { url } = await this.home.machine.directory.locate(
                    place.placement.space,
                    this.clock.now(),
                );

                return { ...place, url };
            },
            serve: async (place, packageId, request) => {
                // address the package below the space's mount over the device's own network
                const url = new URL(request.url);
                const path = `${SpaceMount.path(place.placement.space)}${PackageMount.path(packageId)}`;
                const target = new URL(`${path}${url.pathname}${url.search}`, place.url);
                const headers = new Headers(request.headers);
                headers.set("authorization", `Bearer ${await proven.prove()}`);

                return environment.network.fetch(
                    new Request(target.href, new Request(request, { headers })),
                );
            },
        };

        return new Launcher(machine, proven.session);
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
}
