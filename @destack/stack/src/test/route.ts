import accountPackage from "@destack/account/package";
import { TestDns } from "@destack/account/test";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import type { Simulation } from "@destack/environment/simulation";
import { ClaimLookup, claimTable } from "@destack/object";
import type { StackInput } from "../declare/stack.ts";
import type { TestObjectStore } from "@destack/object/test";
import { found, type Identifier, nth, present, schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { fixtureBuild } from "./build.ts";
import { type FixtureOptions, ids, TestSpace, type HostedSpace } from "./space.ts";

/** The resolver the universe reads DNS over HTTPS from, Cloudflare's. */
const RESOLVER = "cloudflare-dns.com";

/** The port HTTPS serves at. */
const HTTPS_PORT = 443;

/** The DNS response code of an answered lookup (RFC 1035 4.1.1). */
const NO_ERROR = 0;

/** The resource record type of TXT records (RFC 1035 3.2.2). */
const TXT = 16;

/** The domain each account verifies before serving hostnames below it, by account position. */
const DOMAINS = ["example.com", "example.net"] as const;

/** The account of the third space. */
const OTHER_ACCOUNT = schema
    .identifier("account")
    .parse("account-01996ab0-0000-7000-8000-0000000000c1");

/** The spaces the world hosts: two of the fixture's account and one of the other account. */
const SPACES: readonly HostedSpace[] = [
    { id: ids.space, account: ids.account, name: "space" },
    {
        id: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000c2"),
        account: ids.account,
        name: "second",
    },
    {
        id: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000c3"),
        account: OTHER_ACCOUNT,
        name: "rival",
    },
];

/** What the route world's copies share: each space's installation and each account's hostname. */
const RouteWorld = schema.object({
    /** The installation serving notes in each space. */
    installations: schema.array(
        schema.object({
            spaceId: schema.identifier("space"),
            id: schema.identifier("installation"),
        }),
    ),
    /** The hostname the owner created for each account. */
    hostnames: schema.array(
        schema.object({
            accountId: schema.identifier("account"),
            id: schema.identifier("hostname"),
            name: schema.string(),
        }),
    ),
});

/** What the route world's copies share. */
type RouteWorld = schema.Infer<typeof RouteWorld>;

/** The template of the route world, which the run writes once and every route test copies. */
export const ROUTE_TEMPLATE = TestSpace.template(
    {},
    {
        name: "route",
        value: RouteWorld,
        spaces: SPACES,
        write: (_, __, spaces) => TestRoute.write(spaces),
    },
);

/** Two spaces of an account and one of another, each with an installation serving notes. */
export class TestRoute implements AsyncDisposable {
    /** The account of the first two spaces. */
    readonly accountId = ids.account;
    /** The account of the third space. */
    readonly otherId = OTHER_ACCOUNT;
    /** The two spaces of the account and the space of the other account. */
    readonly spaceIds: readonly [Identifier<"space">, Identifier<"space">, Identifier<"space">];
    /** The hostname the owner created for each account, by account. */
    readonly hostnames = new Map<
        string,
        { readonly id: Identifier<"hostname">; readonly name: string }
    >();
    /** The installation of each space, by space. */
    readonly installations: ReadonlyMap<string, Identifier<"installation">>;
    /** The lookup of the universe's names, which keys every route. */
    readonly lookup: ClaimLookup;
    /** The world the spaces run in, which the fixture's calls move through. */
    readonly simulation: Simulation;
    /** Each space with its served store, by space. */
    readonly #spaces: ReadonlyMap<
        string,
        { readonly space: TestSpace; readonly served: TestObjectStore }
    >;

    /** Store the hosted spaces and their installations. */
    private constructor(
        simulation: Simulation,
        spaces: readonly { readonly space: TestSpace; readonly served: TestObjectStore }[],
        installations: ReadonlyMap<string, Identifier<"installation">>,
    ) {
        // store the world, the spaces, their installations, and the universe's names
        this.simulation = simulation;
        this.spaceIds = [nth(spaces, 0).space.id, nth(spaces, 1).space.id, nth(spaces, 2).space.id];
        this.#spaces = new Map(spaces.map((entry) => [entry.space.id, entry]));
        this.installations = installations;
        const { database } = nth(spaces, 0).space.universe.storage;
        this.lookup = ClaimLookup.of(Snapshot.live(database).read(claimTable));
    }

    /** Open the route world's spaces from its template, each served with its notes. */
    static async open(options: FixtureOptions = {}): Promise<TestRoute> {
        // copy the world into a fork of it, then serve each space
        const { spaces, world } = await TestSpace.copy(ROUTE_TEMPLATE);
        const { simulation } = present(spaces.get(ids.space), "the fixture's space");

        return await TestRoute.#serve(simulation, spaces, world, options);
    }

    /** Serve each copied space, with the installations and hostnames the world wrote. */
    static async #serve(
        simulation: Simulation,
        spaces: ReadonlyMap<Identifier<"space">, TestSpace>,
        world: RouteWorld,
        options: FixtureOptions,
    ): Promise<TestRoute> {
        // serve each space
        const served = [];
        for (const space of spaces.values()) {
            served.push({ space, served: await space.serve(options) });
        }

        // read each space's installation and each account's hostname
        const installations = new Map(world.installations.map((each) => [each.spaceId, each.id]));
        const fixture = new TestRoute(simulation, served, installations);
        for (const { accountId, id, name } of world.hostnames) {
            fixture.hostnames.set(accountId, { id, name });
        }

        return fixture;
    }

    /** Write the route world: notes in each space, and a hostname per verified domain. */
    static async write(spaces: ReadonlyMap<Identifier<"space">, TestSpace>): Promise<RouteWorld> {
        // answer the published TXT records at the world's resolver
        const { simulation } = present(spaces.get(ids.space), "the fixture's space");
        const dns = new TestDns();
        simulation.network.server(RESOLVER).http({
            hostname: RESOLVER,
            port: HTTPS_PORT,
            fetch: async (request) => TestRoute.#answer(dns, request),
        });

        // serve each space with notes installed
        const served = [];
        const installations = new Map<Identifier<"space">, Identifier<"installation">>();
        for (const space of spaces.values()) {
            const store = await space.serve();
            const build = await fixtureBuild(ids.package, "2026.9.0", space.environment.crypto);
            installations.set(space.id, await space.install(store, build));
            served.push({ space, served: store });
        }
        const fixture = new TestRoute(simulation, served, installations);

        // verify a domain of each account and serve a hostname below it
        const hostnames = [];
        for (const [index, accountId] of [ids.account, OTHER_ACCOUNT].entries()) {
            const domain = nth(DOMAINS, index);
            await fixture.#verify(dns, accountId, domain);
            const hostname = await fixture.#hostname(accountId, `notes-${index}.${domain}`);
            hostnames.push({ accountId, ...hostname });
        }

        // close the served stores, the template closing the spaces
        for (const each of served) {
            await each.served[Symbol.asyncDispose]();
        }

        return {
            installations: [...installations].map(([spaceId, id]) => ({ spaceId, id })),
            hostnames,
        };
    }

    /** The database of one of the spaces. */
    database(spaceId: string): DatabaseConnection {
        return found(this.#spaces, spaceId).space.database;
    }

    /** Apply a stack's route declarations to a space, approved as its owner. */
    async apply(
        spaceId: Identifier<"space">,
        routes: NonNullable<StackInput["routes"]>,
    ): Promise<void> {
        const { space, served } = found(this.#spaces, spaceId);
        await space.apply(served, [], { routes });
    }

    /** Submit a stack's route declarations to a space, answering how its stack stands. */
    submit(
        spaceId: Identifier<"space">,
        routes: NonNullable<StackInput["routes"]>,
    ): Promise<readonly [string | undefined, string | undefined]> {
        const { space, served } = found(this.#spaces, spaceId);

        return space.submit(served, [], { routes });
    }

    /** Create a hostname of an account as its owner, settled into each space replicating it. */
    serve(
        accountId: Identifier<"account">,
        name: string,
    ): Promise<{ readonly id: Identifier<"hostname">; readonly name: string }> {
        return this.#hostname(accountId, name);
    }

    /** Create a hostname at the universe, and settle the spaces replicating it. */
    async #hostname(
        accountId: Identifier<"account">,
        name: string,
    ): Promise<{ readonly id: Identifier<"hostname">; readonly name: string }> {
        // create the hostname at the universe as the account's owner
        const { space } = found(this.#spaces, this.spaceIds[0]);
        const created = await space.owner.client(accountPackage).hostname.create({
            accountId,
            requestId: RequestId.create(space.universe.environment),
            name,
        });

        // run the world idle while each space follows its account's hostnames
        await this.simulation.idle();

        return { id: created.id, name };
    }

    /** Claim a domain for an account as its owner and verify it by its published TXT record. */
    async #verify(dns: TestDns, accountId: Identifier<"account">, name: string): Promise<void> {
        // let the account's plan allow custom domains, as its residency's home entitles it
        const { space } = found(this.#spaces, this.spaceIds[0]);
        await space.universe.entitle(accountId, "domain", { kind: "static", value: true });

        // claim the domain, then publish its challenge record
        const accounts = space.owner.client(accountPackage);
        const claimed = await accounts.domain.create({
            scope: accountId,
            requestId: RequestId.create(space.universe.environment),
            name,
        });
        dns.publish({ scope: accountId, name });

        // verify it once the resolver answers the record
        await accounts.domain.verify({
            scope: accountId,
            id: claimed.id,
            requestId: RequestId.create(space.universe.environment),
        });
    }

    /** Answer a DNS over HTTPS lookup of TXT records in JSON from the published records. */
    static async #answer(dns: TestDns, request: Request): Promise<Response> {
        const name = new URL(request.url).searchParams.get("name") ?? "";
        const records = await dns.txt(name);

        return Response.json({
            Status: NO_ERROR,
            Answer: records.map((data) => ({ name, type: TXT, data: JSON.stringify(data) })),
        });
    }

    /** Close the spaces in their world. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#close();
    }

    /** Close each space's store, then the space. */
    async #close(): Promise<void> {
        for (const { space, served } of this.#spaces.values()) {
            await served[Symbol.asyncDispose]();
            await space[Symbol.asyncDispose]();
        }
    }
}
