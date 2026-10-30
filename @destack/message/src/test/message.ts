import type { Crypto, Environment } from "@destack/environment";
import type { PermissionReference } from "@destack/access";
import { space } from "@destack/account/object";
import accountPackage from "@destack/account/package";
import type { TestPerson } from "@destack/account/test";
import { eq } from "@destack/db";
import type { Simulation, SimulatedClock } from "@destack/environment/simulation";
import { PACKAGE_DECLARATION, PACKAGE_EXPORT, type PackageHandle } from "@destack/object";
import { describePackage } from "@destack/object/inspect";
import objectPackage from "@destack/object/package";
import type { ObjectStore, PackageImplementation } from "@destack/object/server";
import type { TestCaller, TestObjectStore } from "@destack/object/test";
import { Package } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { RESOURCE_KIND_PACKAGE_ID } from "@destack/resource";
import { type Identifier, present, schema } from "@destack/schema";
import { fixtureOutputs, OWNER_EMAIL, TestPackage, TestSpace, workload } from "@destack/stack/test";
import { testHost } from "@destack/test/host";
import { HOOK_WORKLOAD } from "@destack/stack/workload";
import { ObjectReference, RequestId } from "@destack/sync";
import { defineVault } from "@destack/vault/declare";
import { secret, vault } from "@destack/vault/object";
import vaultPackage from "@destack/vault/package";
import vaultServer from "@destack/vault/server";
import { mailConnector, mailProvider } from "../email/index.ts";
import { describeMail } from "../inspect/index.ts";
import {
    type Content,
    type Destination,
    type Endpoint,
    mail,
    message,
    messageAttempt,
    messageConditions,
    MessageKey,
} from "../object/index.ts";
import messagePackage, { DELIVER_CAPABILITY, outboundMail } from "../package.ts";
import type { Outcome } from "../provider/index.ts";
import messageServer from "../server.ts";
import { documentsPackage, documentsServer } from "./document.ts";
import { type Browser, subscribeBrowser } from "./browser.ts";
import { TestMailTransport } from "./mail.ts";

/** The time every kit's simulation starts at. */
const START = Date.UTC(2026, 9, 5);

/** The provider code the machine provides and connects the space's mail under. */
export const MAIL_PROVIDER = "test";

/** The origin of the webhook endpoint the space's endpoints post to. */
export const HOOKS_ORIGIN = "https://hooks.example.com";

/** The origin of the push service browsers subscribe through. */
export const PUSH_ORIGIN = "https://push.example";

/** The port HTTPS origins listen at (RFC 9110 4.2.2). */
const HTTPS_PORT = 443;

/** The email the member of the space signs up with. */
const MEMBER_EMAIL = "member@example.test";

/** Message as the machine ships it, beside the vault, its mail under the machine's provider. */
const MESSAGE: TestPackage = {
    handle: messagePackage,
    build: await messageBuild(testHost().crypto),
    server: messageServer,
    connectors: [mailConnector(MAIL_PROVIDER, new TestMailTransport())],
    servers: [vaultServer],
    providers: [mailProvider(MAIL_PROVIDER, mail)],
};

/** The documents application, whose calls send messages. */
const DOCUMENTS: TestPackage = {
    handle: documentsPackage,
    build: await MemoryBuild.of(
        Package.of(documentsPackage),
        [packageDeclaration(documentsPackage)],
        testHost().crypto,
    ),
    server: documentsServer,
    connectors: [],
    servers: [],
    providers: [],
};

/** The shipped servers and builds of the packages the space installs, messages before documents. */
const INSTALLED = TestPackage.options([MESSAGE, DOCUMENTS]);

/** The space the kit's tests copy: its installed packages beside the vault's tables, and start. */
const SPACE_OPTIONS = {
    ...INSTALLED,
    packages: [...INSTALLED.packages, vaultPackage],
    start: START,
};

/** The world the kit writes on its space once: the message installation. */
const MessageWorld = schema.object({
    /** The message installation. */
    installation: schema.string(),
});

/** The world the kit writes on its space once. */
type MessageWorld = schema.Infer<typeof MessageWorld>;

/** The vault the space's stack declares for the endpoints' secrets. */
const SECRETS = defineVault({ name: "secrets", spec: {} });

/** A request a receiver answered. */
export interface TestRequest {
    /** The URL it went to. */
    readonly url: string;
    /** Its headers. */
    readonly headers: Headers;
    /** Its body as text. */
    readonly body: string;
}

/** An HTTPS origin on a simulated network, answering each request with the next scripted status. */
export class TestReceiver {
    /** The requests answered, in order. */
    readonly requests: TestRequest[] = [];
    /** The statuses answered in turn, 204 for every request when empty. */
    readonly #statuses: readonly number[];

    /** Answer each request with the next of some statuses, or every one with 204. */
    private constructor(statuses: readonly number[]) {
        this.#statuses = statuses;
    }

    /** Listen at an origin of a simulation's network, as its own node. */
    static listen(simulation: Simulation, origin: string, statuses: readonly number[] = []) {
        // answer each request at the origin's host as a node of that name
        const receiver = new TestReceiver(statuses);
        const { hostname } = new URL(origin);
        simulation.network.server(hostname).http({
            hostname,
            port: HTTPS_PORT,
            fetch: (request) => receiver.#answer(request),
        });

        return receiver;
    }

    /** Record a request and answer the next status. */
    async #answer(request: Request): Promise<Response> {
        // record it with its body
        const { url } = request;
        const headers = new Headers(request.headers);
        this.requests.push({ url, headers, body: await request.text() });

        // answer the next status, with the status as its text
        const status =
            this.#statuses.length === 0
                ? 204
                : present(this.#statuses[this.requests.length - 1], "a status for each request");

        return new Response(null, { status, statusText: `answered ${status}` });
    }
}

/** A message's sending as a test reads it. */
export interface TestSending {
    /** Whether it sends, went out, reached its client, or failed. */
    readonly status: string;
    /** Why it failed, null unless it failed. */
    readonly reason: string | null;
    /** Whether its sealed content is dropped. */
    readonly isErased: boolean;
    /** Each attempt's outcome kind, oldest first. */
    readonly attempts: readonly string[];
    /** Its Sent condition's status and reason, absent before a first attempt. */
    readonly sent?: readonly [string, string];
}

/** The owner's home with message and documents installed, beside a webhook and a push service. */
export class TestMessage {
    /** The space serving messages. */
    readonly space: TestSpace;
    /** The space's object store as its machine serves it. */
    readonly served: TestObjectStore;
    /** The simulation the space's machine runs in. */
    readonly simulation: Simulation;
    /** The simulation's clock. */
    readonly clock: SimulatedClock;
    /** The message installation. */
    readonly installation: Identifier<"installation">;
    /** The space's owner, calling with their client's token. */
    readonly owner: TestCaller;
    /** The transport the space's mail resource connects to. */
    readonly mails: TestMailTransport;
    /** The webhook endpoint at the hooks origin. */
    readonly hooks: TestReceiver;
    /** The push service at the push origin. */
    readonly pushes: TestReceiver;
    /** The secrets the kit created so far, which number the next one's name. */
    #secrets = 0;

    /** The world the space's servers run in, its store's. */
    get environment(): Environment {
        return this.served.environment;
    }

    /** Open a kit over a served space with its installation and outside systems. */
    private constructor(fields: {
        readonly space: TestSpace;
        readonly served: TestObjectStore;
        readonly simulation: Simulation;
        readonly installation: Identifier<"installation">;
        readonly mails: TestMailTransport;
        readonly hooks: TestReceiver;
        readonly pushes: TestReceiver;
    }) {
        // store the space, its installation and the outside systems, and call as its owner
        this.space = fields.space;
        this.served = fields.served;
        this.simulation = fields.simulation;
        this.clock = fields.simulation.clock;
        this.installation = fields.installation;
        this.owner = fields.space
            .person(fields.served, fields.space.owner)
            .driven((work) => fields.simulation.drive(work));
        this.mails = fields.mails;
        this.hooks = fields.hooks;
        this.pushes = fields.pushes;
    }

    /** Serve a space with message and documents installed, outside systems answering in turn. */
    static async open(
        options: {
            /** The outcomes the mail transport answers in turn, every email sent when absent. */
            readonly mails?: readonly Outcome[];
            /** The statuses the webhook endpoint answers in turn, 204 when absent. */
            readonly hooks?: readonly number[];
            /** The statuses the push service answers in turn, 204 when absent. */
            readonly pushes?: readonly number[];
            /** The message server the machine ships, the package's own by default. */
            readonly server?: PackageImplementation;
        } = {},
    ): Promise<TestMessage> {
        // copy the space with message and documents installed, its mail bound to the transport
        const mails = new TestMailTransport(options.mails);
        const { space: hosted, world } = await TestSpace.copy(TestMessage.template(), {
            ...SPACE_OPTIONS,
            servers: [options.server ?? messageServer, documentsServer],
            connectors: [mailConnector(MAIL_PROVIDER, mails)],
        });

        // serve it in its paused world beside the webhook endpoint and the push service
        const { simulation } = hosted;
        const hooks = TestReceiver.listen(simulation, HOOKS_ORIGIN, options.hooks);
        const pushes = TestReceiver.listen(simulation, PUSH_ORIGIN, options.pushes);
        const served = await hosted.serve();
        const installation = schema.identifier("installation").parse(world.installation);

        return new TestMessage({
            space: hosted,
            served,
            simulation,
            installation,
            mails,
            hooks,
            pushes,
        });
    }

    /** Read message as a package a space installs, with the vault and mail it serves beside. */
    static async package(crypto: Crypto): Promise<TestPackage> {
        return { ...MESSAGE, build: await messageBuild(crypto) };
    }

    /** Define the template of the kit's space, which every test copies. */
    static template() {
        return TestSpace.template(SPACE_OPTIONS, {
            name: "message",
            spaces: [{ home: OWNER_EMAIL }],
            value: MessageWorld,
            write: (hosted) => TestMessage.#write(hosted),
        });
    }

    /** Install message and documents in the space, served beside the vault. */
    static async #write(hosted: TestSpace): Promise<MessageWorld> {
        // install message and documents, then stop serving, as the machine does once installations change
        const installing = await hosted.serve();
        const installation = await hosted.install(installing, MESSAGE.build, {
            capabilities: [DELIVER_CAPABILITY],
        });
        await hosted.install(installing, DOCUMENTS.build);
        await installing[Symbol.asyncDispose]();

        return { installation };
    }

    /** A store a provider sends through without reading objects, in an environment. */
    static store(environment: Environment): Pick<ObjectStore, "query" | "environment" | "as"> {
        return {
            query: async () => {
                throw new TypeError("the test reads no objects");
            },
            environment,
            as: () => {
                throw new TypeError("the test serves no objects");
            },
        };
    }

    /** Sign the member in, allowed exactly some permissions through the space's stack. */
    async member(permissions: readonly PermissionReference[]): Promise<TestPerson> {
        const person = await this.space.universe.signIn(MEMBER_EMAIL);
        await this.allow(person, permissions);

        return person;
    }

    /** Grant a person exactly some permissions through a role the space's stack declares. */
    async allow(person: TestPerson, permissions: readonly PermissionReference[]): Promise<void> {
        await this.space.apply(this.served, [], {
            roles: { member: { description: "Use messages", permissions: [...permissions] } },
            relationships: { member: { role: "member", subject: { id: person.id } } },
        });
    }

    /** Seal a message's content to the space's message key, as a producer does. */
    async seal(content: Content): Promise<string> {
        const client = this.owner.client(messagePackage);
        const { key } = await client.message.contentKey({ scope: this.space.id });

        return MessageKey.of(key, this.environment).seal(content, this.space.id);
    }

    /** Send a message to a destination as the owner, sealing any content to the space's key. */
    async send(to: Destination, content?: Content): Promise<Identifier<"message">> {
        const id = message.generateId(this.environment);
        await this.owner.client(messagePackage).message.create({
            ...message.target({ scope: this.space.id, id }),
            requestId: RequestId.create(this.environment),
            to,
            ...ObjectReference.columns(
                "source",
                space.reference(this.space.account, this.space.id),
            ),
            ...(content === undefined ? {} : { ciphertext: await this.seal(content) }),
        });

        return id;
    }

    /** Settle the space's controllers, such as the sends due now. */
    settle(): Promise<void> {
        return this.space.settle(this.served);
    }

    /** Reach a pending message's next attempt, settling what is due then. */
    async retry(id: Identifier<"message">): Promise<void> {
        // read when the message's next attempt is due
        const [row] = await this.served.database
            .select({ retryAt: message.table.retryAt })
            .from(message.table)
            .where(eq(message.table.id, id));
        const retryAt = present(present(row, `message ${id}`).retryAt, `the retry of ${id}`);

        // reach it, at least a moment on, and run until the attempt it was due for, then settle what it changed
        const tried = (await this.sending(id)).attempts.length;
        await this.space.advance(this.served, Math.max(retryAt, this.clock.now() + 1));
        await this.simulation.run({
            until: async () => (await this.sending(id)).attempts.length > tried,
        });
        await this.settle();
    }

    /** Read a message's sending: its status, its erasure, its attempts and its Sent condition. */
    async sending(id: Identifier<"message">): Promise<TestSending> {
        // read the message and its attempts
        const { database } = this.served;
        const [row] = await database.select().from(message.table).where(eq(message.table.id, id));
        const found = present(row, `message ${id}`);
        const attempts = await database
            .select({ outcome: messageAttempt.table.outcome })
            .from(messageAttempt.table)
            .where(eq(messageAttempt.table.parentId, id))
            .orderBy(messageAttempt.table.createdAt, messageAttempt.table.id);

        // read the Sent condition's status and reason
        const sent = messageConditions.read(found, "Sent");

        return {
            status: found.status,
            reason: found.reason,
            isErased: found.ciphertext === null,
            attempts: attempts.map(({ outcome }) => outcome.kind),
            ...(sent === undefined ? {} : { sent: [sent.status, sent.reason] as const }),
        };
    }

    /** Subscribe an owner's browser to pushes at a path of the push service. */
    async browser(path: string): Promise<{
        readonly to: Extract<Destination, { readonly channel: "push" }>;
        readonly browser: Browser;
    }> {
        // create the owner's browser client at their account, and store its subscription
        const browser = await subscribeBrowser(this.environment);
        const { owner } = this.space;
        const clients = owner.client(accountPackage).client;
        const requestId = () => RequestId.create(this.environment);
        const created = await this.simulation.drive(
            clients.create({
                userId: owner.id,
                requestId: requestId(),
                name: "Firefox",
                kind: "browser",
            }),
        );
        const push = { url: `${PUSH_ORIGIN}${path}`, keys: browser.keys };
        await this.simulation.drive(
            clients.subscribe({ userId: owner.id, id: created.id, requestId: requestId(), push }),
        );

        return { to: { channel: "push", client: created.id, user: owner.id }, browser };
    }

    /** Store a text as a secret of the vault the space's stack declares, as its owner writes it. */
    async secret(value: string): Promise<ObjectReference> {
        // declare the vault through the space's stack
        await this.space.apply(this.served, [vaultPackage], {
            vaults: { secrets: SECRETS.declare({ retention: "delete" }) },
        });
        const client = this.owner.client(vaultPackage);
        const [declared] = await this.served.database
            .select({ id: vault.table.id })
            .from(vault.table);

        // create the secret with the value as its first version, current
        const created = await client.secret.create({
            scope: this.space.id,
            parentId: present(declared, "the space's vault").id,
            name: `secret-${++this.#secrets}`,
            requestId: RequestId.create(this.environment),
        });
        const version = await client.version.create({
            scope: this.space.id,
            parentId: created.id,
            requestId: RequestId.create(this.environment),
            value: { encoding: "text", value },
        });
        await client.secret.update({
            scope: this.space.id,
            id: created.id,
            requestId: RequestId.create(this.environment),
            currentVersion: version.number,
        });

        return secret.reference(this.space.id, created.id);
    }

    /** Create an endpoint posting to the webhook endpoint as a caller, the owner by default. */
    async subscribe(
        fields: Pick<Endpoint, "events" | "format" | "authentication" | "secret">,
        caller = this.owner,
    ): Promise<Identifier<"endpoint">> {
        const created = await caller.client(messagePackage).endpoint.create({
            scope: this.space.id,
            requestId: RequestId.create(this.environment),
            url: `${HOOKS_ORIGIN}/destack`,
            ...fields,
        });

        return created.id;
    }
}

/** Write message's build: its package, its outbound mail and the workload serving its hooks. */
function messageBuild(crypto: Crypto) {
    const outputs = fixtureOutputs({
        server: {
            runtime: "bun",
            emit: true,
            workloads: { [HOOK_WORKLOAD]: workload },
            views: {},
        },
    });

    return MemoryBuild.of(
        Package.of(messagePackage),
        [
            packageDeclaration(messagePackage),
            {
                kind: "resource",
                kindPackageId: RESOURCE_KIND_PACKAGE_ID,
                name: outboundMail.name,
                description: describeMail(outboundMail),
            },
        ],
        crypto,
        { outputs },
    );
}

/** Describe a package as its build's package declaration. */
function packageDeclaration(handle: PackageHandle) {
    return {
        kind: PACKAGE_DECLARATION,
        kindPackageId: objectPackage.id,
        name: PACKAGE_EXPORT,
        description: describePackage(handle),
    };
}
