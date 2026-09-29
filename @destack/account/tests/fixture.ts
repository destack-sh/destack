import { principal, type Restriction, type Subject } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { Scope } from "@destack/sync";
import type { TestDatabase } from "@destack/db/test";
import { ResourceContext } from "@destack/resource/context";
import { identifier } from "@destack/schema";
import { Health } from "@destack/service/health";
import { RequestId } from "@destack/service/request";
import { Server } from "@destack/service/server";
import { Caller } from "@destack/service/authentication";
import { DirectoryDatabase } from "@destack/directory";
import { v7 } from "uuid";
import { accountPackage } from "../src/audit/index.ts";
import {
    AccountCaller,
    createAuthentication,
    type Authentication,
    type AuthenticationOptions,
} from "../src/authentication/index.ts";
import { connect } from "../src/client/client.ts";
import {
    Connections,
    implementService,
    type ConnectionCallback,
    type ConnectionGrant,
    type ConnectionProvider,
    type ConnectionRequest,
    type ConnectionRevocation,
    type ConnectionVault,
} from "../src/server/index.ts";
import type { User } from "../src/object/user.ts";
import { account } from "../src/object/account.ts";
import { session } from "../src/object/authentication.ts";
import { Digest } from "../src/object/digest.ts";
import { eq } from "@destack/db";
import type { AuditEvent } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import { Browser } from "./browser.ts";
import { openAccountDatabase } from "./database.ts";

/** The origin the fixture's account service answers on. */
const ORIGIN = "http://localhost:3210";

/** The header with the host a test request acts as, in place of a host proof. */
const HOST_HEADER = "x-host";

/** How long the outbox takes at most to deliver, in milliseconds. */
const DELIVERY_MILLISECONDS = 5000;

/** How long a host's verified caller lasts: an hour, beyond every test, in milliseconds. */
const HOST_MILLISECONDS = 60 * 60 * 1000;

/** A scope with its own database and a name unique in the account, like a space. */
export const place = defineObject({
    name: "place",
    identity: "space",
    plural: "places",
    scope: account,
    isScope: true,
    fields: { name: field.string() },
    indexes: { name: { on: ["name"], unique: true, across: account } },
    permissions: ["read"],
});

/** A migrated account database served over HTTP, with delivered sign-in messages and connection fakes. */
export class AccountFixture implements AsyncDisposable {
    /** The isolated database. */
    readonly opened: TestDatabase;
    /** The platform authentication over the database. */
    readonly authentication: Authentication;
    /** The account service. */
    readonly server: Server;
    /** The sign-in links delivered so far. */
    readonly links: { email: string; url: string }[];
    /** The email codes delivered so far. */
    readonly codes: { email: string; otp: string; type: string }[];
    /** The vault holding connections' credentials. */
    readonly vault: MemoryVault;
    /** The connection providers, by name. */
    readonly providers: ReadonlyMap<string, MemoryProvider>;
    /** The verified callers of the hosts tests act as, by host. */
    readonly hosts: Map<string, Caller>;
    /** The audit events the outbox delivered. */
    readonly audited: AuditEvent[];

    /** Hold an opened, migrated and served database. */
    private constructor(
        opened: TestDatabase,
        authentication: Authentication,
        server: Server,
        delivered: Pick<
            AccountFixture,
            "links" | "codes" | "vault" | "providers" | "hosts" | "audited"
        >,
    ) {
        this.opened = opened;
        this.authentication = authentication;
        this.server = server;
        this.links = delivered.links;
        this.codes = delivered.codes;
        this.vault = delivered.vault;
        this.providers = delivered.providers;
        this.hosts = delivered.hosts;
        this.audited = delivered.audited;
    }

    /** Open, migrate and serve an account database, with the sign-in options a test changes. */
    static async open(options: Partial<AuthenticationOptions> = {}): Promise<AccountFixture> {
        // open an isolated, migrated database
        const opened = await openAccountDatabase();

        // configure sign-in delivering its messages to the fixture
        const links: AccountFixture["links"] = [];
        const codes: AccountFixture["codes"] = [];
        const authentication = createAuthentication({
            database: opened.database,
            origin: ORIGIN,
            trustedOrigins: [ORIGIN],
            ipAddress: { disableIpTracking: true },
            secret: "local-account-test-secret-32-characters-minimum",
            providers: {},
            signInUri: `${ORIGIN}/sign-in`,
            consentUri: `${ORIGIN}/consent`,
            verificationUri: `${ORIGIN}/device`,
            secondFactorUri: `${ORIGIN}/sign-in/two-factor`,
            handleUri: `${ORIGIN}/sign-in/handle`,
            service: (request) => server.fetch(request),
            sendMagicLink: async (message) => {
                links.push(message);
            },
            sendCode: async (message) => {
                codes.push(message);
            },
            ...options,
        });

        // serve connections to an OAuth provider and a provider installing its application
        const vault = new MemoryVault();
        const providers = new Map(
            [
                new MemoryProvider("github", "oauth"),
                new MemoryProvider("github-app", "installation"),
            ].map((provider) => [provider.name, provider]),
        );
        const connections = new Connections({ providers: [...providers.values()], vault });

        // keep the delivered audit events
        const audited: AuditEvent[] = [];
        const history = {
            ingest: async (batch: { readonly events: readonly AuditEvent[] }) => {
                audited.push(...batch.events);
            },
        };

        // verify the hosts a test enrolls, and everyone else by their account credential
        const hosts = new Map<string, Caller>();
        const server = Server.start({
            ...implementService(authentication, { connections, history, inherited: [] }),
            audience: accountPackage.id,
            resources: new ResourceContext(),
            health: new Health("account"),
            authenticate: async (request) => {
                const host = request.headers.get(HOST_HEADER);

                return host === null
                    ? AccountCaller.authenticate(request, authentication)
                    : hosts.get(host)!;
            },
            authorizeHost: async () => {},
            drainTimeout: 1000,
        });

        return new AccountFixture(opened, authentication, server, {
            links,
            codes,
            vault,
            providers,
            hosts,
            audited,
        });
    }

    /** Wait until the outbox delivered every event, returning the delivered events. */
    async delivered(): Promise<readonly AuditEvent[]> {
        const outbox = new AuditOutbox(this.opened.database);
        await this.opened.database.log.until(
            async () => (await outbox.read()).length === 0,
            AbortSignal.timeout(DELIVERY_MILLISECONDS),
        );

        return this.audited;
    }

    /** Sign a person in through a delivered link in a browser of its own. */
    async signIn(email: string): Promise<Person> {
        // follow the delivered link
        const browser = this.browser();
        await browser.fetch("/auth/sign-in/magic-link", { email, name: email, callbackURL: "/" });
        await browser.fetch(this.links.at(-1)!.url);

        // read the signed-in user through the browser's cookies
        const client = this.connect(() => ({
            origin: ORIGIN,
            cookie: [...browser.cookies].map(([name, value]) => `${name}=${value}`).join("; "),
        }));
        const { subject } = await client.authentication.current();
        const id = identifier("user").parse(subject.id);

        return { browser, client, id, subject: principal.user.reference(Scope.universe.id, id) };
    }

    /** Open a browser of the account service. */
    browser(): Browser {
        return new Browser(ORIGIN, (request) => this.server.fetch(request));
    }

    /** Record a second factor as the last sign-in ceremony of a person's sessions for elevation. */
    async elevate(person: Person): Promise<void> {
        await this.opened.database
            .update(session.table)
            .set({ authenticationMethod: "totp", authenticatedAt: Date.now() })
            .where(eq(session.table.userId, person.id));
    }

    /** Create an account of a person with one space a region serves. */
    async createSpace(person: Person) {
        // choose a residency before the handle claims the personal account
        const { profile } = await person.client.authentication.current();
        if (profile.handle === null) {
            const current = await person.client.user.get({ id: person.id });
            await person.client.user.update({
                id: person.id,
                requestId: RequestId.create(),
                revision: current.revision,
                residency: "eu",
            });
        }

        // create the account
        const created = await person.client.account.create({
            kind: profile.handle === null ? "personal" : "shared",
            scope: person.id,
            requestId: RequestId.create(),
            handle: `account-${v7().slice(-12)}`,
            name: "Account",
            defaultResidency: "eu",
        });

        // place a space in it in a region
        const spaceId = identifier("space").parse(`space-${v7()}`);
        const regionId = identifier("region").parse(`region-${v7()}`);
        await new DirectoryDatabase(this.opened.database).place({
            id: spaceId,
            scope: created.id,
            cell: regionId,
            epoch: 1,
        });

        return { accountId: created.id, handle: created.handle, spaceId, regionId };
    }

    /** Enroll a host of an account, serving a region when given, and connect a client acting as it. */
    host(accountId: string, regionId?: string): Host {
        // verify the host as itself, and as its region's
        const id = `host-${v7()}`;
        const subject = principal.host.reference(accountId, id);
        const regions =
            regionId === undefined ? [] : [principal.region.reference(Scope.universe.id, regionId)];
        const now = Date.now();
        this.hosts.set(
            id,
            new Caller({
                credential: { kind: "host-key", id },
                audience: accountPackage.id,
                verifiedAt: now,
                expiresAt: now + HOST_MILLISECONDS,
                subject,
                subjects: [subject, ...regions],
            }),
        );

        return { id, subject, client: this.connect(() => ({ [HOST_HEADER]: id })) };
    }

    /** Connect a typed client sending the given headers. */
    connect(headers: () => Record<string, string>): ReturnType<typeof connect> {
        return connect({ url: ORIGIN, headers, fetch: (request) => this.server.fetch(request) });
    }

    /** Stop the service, then close the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.server.close();
        await this.opened.close();
    }
}

/** A signed-in person: its browser, its typed client and its user. */
export interface Person {
    /** The browser holding the session cookie. */
    readonly browser: Browser;
    /** The typed client sending the browser's cookies. */
    readonly client: ReturnType<typeof connect>;
    /** The user's identifier. */
    readonly id: User["id"];
    /** The user as a principal. */
    readonly subject: Subject;
}

/** A host the fixture verifies: its identity, principal and typed client. */
export interface Host {
    /** The host's identity. */
    readonly id: string;
    /** The host as a principal. */
    readonly subject: Subject;
    /** The typed client acting as the host. */
    readonly client: ReturnType<typeof connect>;
}

/** An in-memory provider that approves its own authorizations and checks their PKCE challenge. */
export class MemoryProvider implements ConnectionProvider {
    /** The provider's name. */
    readonly name: string;
    /** The provider instance. */
    readonly issuer: string;
    /** The provider application. */
    readonly applicationId = "destack-test";
    /** Whether the provider grants OAuth credentials or installs the application. */
    readonly kind: "oauth" | "installation";
    /** The authorizations withdrawn so far. */
    readonly revoked: ConnectionRevocation[] = [];
    /** The approved callbacks awaiting their exchange: their challenge and account, by code or installation. */
    readonly #approved = new Map<string, { challenge: string; subject: string }>();

    /** Create a provider of one kind. */
    constructor(name: string, kind: "oauth" | "installation") {
        this.name = name;
        this.issuer = `https://${name}.test`;
        this.kind = kind;
    }

    /** Build the provider page carrying the state, the challenge and the scopes. */
    authorize(request: ConnectionRequest): URL {
        const page = new URL("/authorize", this.issuer);
        page.searchParams.set("state", request.state);
        page.searchParams.set("code_challenge", request.challenge);
        page.searchParams.set("scope", request.scopes.join(" "));

        return page;
    }

    /** Approve an authorization page as an external account, returning the callback parameters. */
    approve(page: string, subject: string): Record<string, string> {
        const url = new URL(page);
        const key = v7();
        this.#approved.set(key, { challenge: url.searchParams.get("code_challenge")!, subject });
        const parameter = this.kind === "oauth" ? "code" : "installation_id";

        return { [parameter]: key, state: url.searchParams.get("state")! };
    }

    /** Exchange an approved callback once, requiring the verifier of its challenge. */
    async complete(callback: ConnectionCallback): Promise<ConnectionGrant> {
        // require an approved, unused callback and its verifier
        const key = callback.parameters.code ?? callback.parameters.installation_id!;
        const approved = this.#approved.get(key);
        const challenge = await Digest.base64url(callback.verifier);
        if (approved === undefined || approved.challenge !== challenge) {
            throw new Error("invalid grant");
        }
        this.#approved.delete(key);

        // grant a credential, or the installation
        return this.kind === "oauth"
            ? {
                  subject: approved.subject,
                  scopes: callback.scopes,
                  permissions: {},
                  credential: `credential-of-${approved.subject}`,
              }
            : {
                  subject: approved.subject,
                  scopes: [],
                  permissions: { contents: "read" },
                  installationId: key,
              };
    }

    /** Record the withdrawn authorization. */
    async revoke(grant: ConnectionRevocation): Promise<void> {
        this.revoked.push(grant);
    }
}

/** Vaults holding secrets in memory, as the account service reaches them. */
export class MemoryVault implements ConnectionVault {
    /** The held secrets, by identifier. */
    readonly secrets = new Map<
        string,
        { spaceId: string; vaultId: string; name: string; value: string; subject: Subject }
    >();

    /** Keep a secret under its identifier. */
    async write(secret: {
        readonly id: string;
        readonly spaceId: string;
        readonly vaultId: string;
        readonly name: string;
        readonly value: string;
        readonly subject: Subject;
    }): Promise<void> {
        const { id, ...held } = secret;
        this.secrets.set(identifier("secret").parse(id), held);
    }

    /** Read a held secret of a space. */
    async read(secret: { readonly spaceId: string; readonly secretId: string }): Promise<string> {
        const held = this.secrets.get(secret.secretId);
        if (held === undefined || held.spaceId !== secret.spaceId) {
            throw new Error("secret not found");
        }

        return held.value;
    }

    /** Forget a held secret. */
    async destroy(secret: { readonly secretId: string }): Promise<void> {
        this.secrets.delete(secret.secretId);
    }
}

/** Read who a caller is from its own authentication: its subject, credential and profile. */
export function identity(current: {
    readonly subject: unknown;
    readonly credential: unknown;
    readonly profile: unknown;
}) {
    return { subject: current.subject, credential: current.credential, profile: current.profile };
}

/** Report a call's outcome as its failure code and message, or as executed. */
export function outcome(call: Promise<unknown>): Promise<string> {
    return call.then(
        () => "executed",
        (error: { code: string; message: string }) => `${error.code}: ${error.message}`,
    );
}

/** Allocate a typed UUIDv7 for an isolated fixture record. */
export function id<const Prefix extends string>(prefix: Prefix) {
    return identifier(prefix).parse(`${prefix}-${v7()}`);
}

/** Read the caller an exchanged access token asserts. */
export function claims(token: string): {
    readonly subject: Subject;
    readonly delegates?: readonly { readonly subject: Subject; readonly authority: string }[];
    readonly permissions?: readonly Restriction[];
} {
    const [, payload] = token.split(".");
    const decoded = new TextDecoder().decode(
        Uint8Array.fromBase64(payload!, { alphabet: "base64url" }),
    );

    return JSON.parse(decoded).caller;
}
