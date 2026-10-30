import type { Environment, Crypto } from "@destack/environment";
import { type PermissionReference, principal } from "@destack/access";
import * as account from "@destack/account/object";
import type { TestPerson } from "@destack/account/test";
import type { Simulation, SimulatedClock } from "@destack/environment/simulation";
import { notification } from "@destack/notification";
import { TestNotification } from "@destack/notification/test";
import { testHost } from "@destack/test/host";
import { CallError, PACKAGE_DECLARATION, PACKAGE_EXPORT } from "@destack/object";
import { describePackage } from "@destack/object/inspect";
import objectPackage from "@destack/object/package";
import type { TestCaller, TestObjectStore } from "@destack/object/test";
import { Package } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { type Identifier, present, schema } from "@destack/schema";
import { describeSetting } from "@destack/setting/inspect";
import settingPackage from "@destack/setting/package";
import socialPackage from "@destack/social/package";
import type { StackDocument, StackTypes } from "@destack/stack/declare";
import { TestPackage, TestSpace } from "@destack/stack/test";
import { Scope } from "@destack/sync";
import { FRAMES_ATTRIBUTE, type OtlpSignal } from "@destack/telemetry/otlp";
import {
    alert,
    alertRule,
    firingAlert,
    issue,
    regressedIssue,
    resolvedAlert,
    resolvedIssue,
    RESPOND_CAPABILITY,
    silence,
} from "../object/index.ts";
import { EMITTER_ATTRIBUTES } from "../otlp/index.ts";
import { analyticsRetention, telemetryRetention, traceSampling } from "../setting/index.ts";
import observabilityPackage from "../package.ts";
import observabilityServer from "../server.ts";
import { type TestException, TestEmitter } from "./emitter.ts";
import { NOTES, RENAME_STACK, TestNotes } from "./notes.ts";

/** How long the issues reader waits to take a page: its one minute batch, and a second to run. */
const ISSUE_COUNT_MILLISECONDS = 61_000;

/** When the kits' simulations start by default: 16 October 2026, UTC. */
export const START = Date.UTC(2026, 9, 16);

/** The packages notifications install with, in dependency order. */
const NOTIFICATIONS = await TestNotification.packages(testHost().crypto);

/** Observability, as the machine ships it. */
const OBSERVABILITY: TestPackage = {
    handle: observabilityPackage,
    build: await observabilityBuild(testHost().crypto),
    server: observabilityServer,
    connectors: [],
    servers: [],
    providers: [],
};

/** The packages the space installs, in dependency order. */
export const PACKAGES: readonly TestPackage[] = [...NOTIFICATIONS, OBSERVABILITY];

/** The origin the test's exports address, which no request leaves the process for. */
const OTLP_ORIGIN = "https://observability.test";

/** The email the member of the space signs up with. */
const MEMBER_EMAIL = "member@example.test";

/** The workload instance of the notes installation that emits the exports. */
const INSTANCE = schema
    .identifier("instance")
    .parse("instance-01996ab0-0000-7000-8000-000000000006");

/** The world the kit writes on its space once: the notes application's installation. */
const ObservabilityWorld = schema.object({
    /** The notes application's installation. */
    notes: schema.string(),
});

/** The world the kit writes on its space once. */
type ObservabilityWorld = schema.Infer<typeof ObservabilityWorld>;

/** The shipped servers and builds of the packages the space installs. */
const INSTALLED = TestPackage.options(PACKAGES);

/** The space the kit's tests copy: its installed packages beside social, for comments on issues. */
const SPACE_OPTIONS = { ...INSTALLED, packages: [...INSTALLED.packages, socialPackage] };

/** Every permission an observer of the space's issues, alerts and silences needs. */
export const OBSERVER_PERMISSIONS: readonly PermissionReference[] = [
    issue.permission("read"),
    issue.permission("write"),
    alertRule.permission("read"),
    alertRule.permission("write"),
    alertRule.permission("manage"),
    alert.permission("read"),
    alert.permission("write"),
    silence.permission("read"),
    silence.permission("write"),
    {
        packageId: account.space.policy.definition.packageId,
        type: account.space.name,
        name: "read",
    },
];

/** A space installing observability and the notes application, observed by a member. */
export class TestObservability implements AsyncDisposable {
    /** The space serving observability. */
    readonly space: TestSpace;
    /** The served space, served anew on each restart. */
    #served: TestObjectStore;
    /** The simulation the space runs on, its clock moving only as tests run it. */
    readonly simulation: Simulation;
    /** The notes application's installation. */
    readonly notes: Identifier<"installation">;
    /** The notes installation's workload, exporting through the space's machine. */
    readonly emitter: TestEmitter;
    /** The member of the space, allowed what the test grants. */
    readonly #member: TestPerson;
    /** The attributed frames of a `Note.rename` failure, as the machine forwards them. */
    readonly #renameFrames: string;

    /** The served space. */
    get served(): TestObjectStore {
        return this.#served;
    }

    /** The world the space's servers run in, its store's. */
    get environment(): Environment {
        return this.served.environment;
    }

    /** The member of the space, allowed what the test grants, calling under the simulation. */
    get member(): TestCaller {
        return this.person(this.#member);
    }

    /** The space's machine, forwarding its workloads' exports. */
    get machine(): TestCaller {
        return this.served.as(
            principal.machine.reference(Scope.universe.id, this.space.machine.id),
        );
    }

    /** Keep a served space with its installations and member. */
    private constructor(fields: {
        readonly space: TestSpace;
        readonly served: TestObjectStore;
        readonly simulation: Simulation;
        readonly notes: Identifier<"installation">;
        readonly emitter: TestEmitter;
        readonly member: TestPerson;
        readonly renameFrames: string;
    }) {
        // keep the space and how it is served, and address the member and the machine
        this.space = fields.space;
        this.#served = fields.served;
        this.simulation = fields.simulation;
        this.notes = fields.notes;
        this.emitter = fields.emitter;
        this.#member = fields.member;
        this.#renameFrames = fields.renameFrames;
    }

    /** Serve a space with observability responding to alerts, notes installed, and an observer. */
    static async open(start = START): Promise<TestObservability> {
        // copy the space with its installations and observing member into a fork of its world
        const template = TestObservability.template(start);
        const { space, world } = await TestSpace.copy(template, { ...SPACE_OPTIONS, start });

        // serve it, driving the work through the fork
        return TestObservability.#open(space, world);
    }

    /** Serve a copied space to its member. */
    static async #open(space: TestSpace, world: ObservabilityWorld): Promise<TestObservability> {
        // serve the space with notifications, read by its member
        const { simulation } = space;
        const served = await space.serve();
        const member = space.universe.person(MEMBER_EMAIL);

        // emit as the notes installation's workload, its build's frames attributed
        const notes = schema.identifier("installation").parse(world.notes);
        const manifest = await space.registry.find(NOTES.id, NOTES.version);
        const emitter = new TestEmitter(
            {
                "service.name": NOTES.name,
                [EMITTER_ATTRIBUTES.installation]: notes,
                [EMITTER_ATTRIBUTES.instance]: INSTANCE,
                [EMITTER_ATTRIBUTES.build]: manifest,
            },
            { name: NOTES.name, version: NOTES.version },
        );
        const renameFrames = await TestNotes.frames(await TestNotes.build());

        return new TestObservability({
            space,
            served,
            simulation,
            notes,
            emitter,
            member,
            renameFrames,
        });
    }

    /** Define the template of the kit's space at a start, which every test at that start copies. */
    static template(start = START) {
        return TestSpace.template(
            { ...SPACE_OPTIONS, start },
            {
                name: "observability",
                value: ObservabilityWorld,
                write: (space) => TestObservability.#write(space),
            },
        );
    }

    /** Install observability and the notes application, and sign up a member allowed to observe. */
    static async #write(space: TestSpace): Promise<ObservabilityWorld> {
        // install notifications, observability allowed to respond to alerts, and the notes application
        const installing = await space.serve();
        for (const installed of NOTIFICATIONS) {
            await space.install(installing, installed.build);
        }
        await space.install(installing, OBSERVABILITY.build, {
            capabilities: [RESPOND_CAPABILITY],
        });
        const notes = await space.install(installing, await TestNotes.build());
        await installing[Symbol.asyncDispose]();

        // serve again, as the machine does once installations change, and let a member observe
        await using served = await space.serve();
        const member = await space.universe.signIn(MEMBER_EMAIL);
        await space.apply(served, [], TestObservability.#observer(member.id, OBSERVER_PERMISSIONS));

        return { notes };
    }

    /** Declare a role granting a member exactly some permissions in the space's stack. */
    static #observer(member: Identifier<"user">, permissions: readonly PermissionReference[]) {
        return {
            roles: {
                observer: { description: "Observe the space", permissions: [...permissions] },
            },
            relationships: { member: { role: "observer", subject: { id: member } } },
        };
    }

    /** The digest of the notes build's manifest, as its revisions record it. */
    get manifest(): string {
        return String(this.emitter.resource[EMITTER_ATTRIBUTES.build]);
    }

    /** Capture a failure of `Note.rename` for a person, as the notes workload exports it. */
    renameFailure(time: number, person: string): TestException {
        return {
            time,
            attributes: {
                "error.type": "RangeError",
                "exception.type": "RangeError",
                "exception.message": "title is too long\nat most 500 characters",
                "exception.stacktrace": RENAME_STACK,
                [FRAMES_ATTRIBUTE]: this.#renameFrames,
                "exception.escaped": true,
                "enduser.id": person,
            },
        };
    }

    /** Address the space as a person, each call run under the simulation. */
    person(person: TestPerson): TestCaller {
        return this.space.person(this.served, person);
    }

    /** Apply a stack document declaring records of some types to the space under the simulation. */
    apply<const Types extends StackTypes>(
        types: Types,
        document: StackDocument<Types>,
    ): Promise<void> {
        return this.space.apply(this.served, types, document);
    }

    /** Submit a stack document to the space, answering how its stack stands. */
    submit<const Types extends StackTypes>(
        types: Types,
        document: StackDocument<Types>,
    ): Promise<readonly [string | undefined, string | undefined]> {
        return this.space.submit(this.served, types, document);
    }

    /** Grant the member exactly some permissions through a role the space's stack declares. */
    async allow(permissions: readonly PermissionReference[]): Promise<void> {
        await this.apply([], TestObservability.#observer(this.#member.id, permissions));
    }

    /** Post an OTLP/JSON export to observability's adapter, as the machine by default. */
    async post(signal: OtlpSignal, exported: unknown, caller = this.machine): Promise<number> {
        // send the export to the adapter as the caller
        const response = await caller.endpoint(observabilityPackage).fetch(
            new Request(`${OTLP_ORIGIN}/v1/${signal}`, {
                method: "POST",
                headers: { "content-type": "application/json" },
                body: JSON.stringify(exported),
            }),
        );

        // throw the adapter's refusal as the call error its body describes, else read its export response
        const answered: unknown = await response.json();
        if (!response.ok) {
            throw present(CallError.fromJson(answered), `the refusal of an OTLP ${signal} export`);
        }

        return response.status;
    }

    /** Run past the issues reader's minute, counting the posted exceptions into issues. */
    async count(): Promise<void> {
        await this.simulation.clock.sleep(
            this.clock.now() + ISSUE_COUNT_MILLISECONDS - this.simulation.clock.now(),
        );
        await this.simulation.idle();
    }

    /** The clock the space's servers read. */
    get clock(): SimulatedClock {
        return this.simulation.clock;
    }

    /** Serve the space anew, as its machine does once its installations change. */
    async restart(): Promise<void> {
        await this.#served[Symbol.asyncDispose]();
        this.#served = await this.space.serve();
    }

    /** Stop serving the space, then close it, running the simulation while they wind down. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.served[Symbol.asyncDispose]();
        await this.space[Symbol.asyncDispose]();
    }
}

/** Write observability's build: its package, its settings and its notifications. */
function observabilityBuild(crypto: Crypto) {
    return MemoryBuild.of(
        Package.of(observabilityPackage),
        [
            {
                kind: PACKAGE_DECLARATION,
                kindPackageId: objectPackage.id,
                name: PACKAGE_EXPORT,
                description: describePackage(observabilityPackage),
            },
            ...[telemetryRetention, analyticsRetention, traceSampling].map((declared) => ({
                kind: "setting",
                kindPackageId: settingPackage.id,
                name: declared.name,
                description: describeSetting(declared),
            })),
            ...[regressedIssue, resolvedIssue, firingAlert, resolvedAlert].map((declared) => ({
                kind: "notification",
                kindPackageId: notification.package.id,
                name: declared.name,
                description: declared.describe(),
            })),
        ],
        crypto,
    );
}
