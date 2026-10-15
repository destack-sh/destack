import { account, hostname, space } from "@destack/account/object";
import { Scope } from "@destack/sync";
import { type DatabaseConnection, type Dialect, Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Manager } from "@destack/access";
import { type ClaimHome, Stack } from "@destack/object/server";
import { aligned, Clock, Identifier, type JsonObject } from "@destack/schema";
import { installation } from "../object/index.ts";
import { type ObjectType, Claim, Claimant, ClaimLookup, claimTable } from "@destack/object";
import { STACKS, TestDirectory } from "@destack/account/test";
import type { ObjectStore } from "@destack/object/server";
import { serveSpace } from "../server/index.ts";
import { StackFixture } from "./fixture.ts";
import { accountDatabase } from "@destack/account/stack";
import type {} from "@destack/package/import-meta";

/** The package declaring the fixture's stacks and installations. */
const PACKAGE_ID = import.meta.destack.package.id;

/** Two spaces of an account and one of another. */
export class RouteFixture implements AsyncDisposable {
    /** The account owning the first two spaces. */
    readonly accountId = Identifier.create("account");
    /** The account owning the third space. */
    readonly otherId = Identifier.create("account");
    /** The two spaces of the account and the space of the other account. */
    readonly spaceIds = [
        Identifier.create("space"),
        Identifier.create("space"),
        Identifier.create("space"),
    ] as const;
    /** The machine's replicated hostname of each account, by account. */
    readonly hostnames = new Map<
        string,
        { readonly id: Identifier<"hostname">; readonly name: string }
    >();
    /** The installation of each space, by space. */
    readonly installations = new Map<string, Identifier<"installation">>();
    /** The stack installation declaring each space's routes, by space. */
    readonly #stacks = new Map<string, string>();
    /** The machine keeping the spaces. */
    readonly #spaces: StackFixture;
    /** The machine's object store applying the declared records. */
    readonly #served: ObjectStore;
    /** The installations and routes the stacks declare as the machine serves them. */
    readonly #declared: readonly ObjectType[];
    /** The universe's test database. */
    readonly #universe: TestDatabase;
    /** The universe's claims, served over its database. */
    readonly claims: ClaimHome;
    /** The lookup of the universe's names over its database. */
    readonly lookup: ClaimLookup;

    /** Keep the opened databases. */
    private constructor(
        spaces: StackFixture,
        served: ObjectStore,
        universe: TestDatabase,
        claims: ClaimHome,
    ) {
        // keep the databases, the machine's store and the installations and routes it declares
        this.#spaces = spaces;
        this.#served = served;
        this.#declared = served.objects.filter(
            (type) => type.name === "installation" || type.name === "route",
        );
        this.#universe = universe;
        this.claims = claims;
        this.lookup = ClaimLookup.of(Snapshot.live(universe.database).read(claimTable));
    }

    /** The spaces database. */
    get spaces(): DatabaseConnection {
        return this.#spaces.database;
    }

    /** The universe's database with the claims. */
    get universe(): DatabaseConnection {
        return this.#universe.database;
    }

    /** Open both databases on a dialect and record the spaces and their installations. */
    static async open(dialect: Dialect): Promise<RouteFixture> {
        // open both databases and declare installations and routes as the machine serves them
        const spaces = await StackFixture.empty({ dialect });
        const served = serveSpace(await spaces.options()).store;
        const universe = await TestDatabase.create(dialect, accountDatabase, { isMigrated: true });
        const claims = await TestDirectory.claims(universe.database, Clock.system);
        const fixture = new RouteFixture(spaces, served, universe, claims);

        // record the accounts with their hostnames, and the spaces with their installations
        const now = Date.now();
        const record = { createdAt: now, updatedAt: now };
        const ownerId = Identifier.create("user");
        await fixture.#recordAccounts(ownerId, record);
        await fixture.#recordSpaces(ownerId, record);

        return fixture;
    }

    /** Record each account's scope under its owner and a hostname of it. */
    async #recordAccounts(
        ownerId: Identifier<"user">,
        record: { readonly createdAt: number; readonly updatedAt: number },
    ): Promise<void> {
        for (const [index, accountId] of [this.accountId, this.otherId].entries()) {
            await this.spaces.insert(Scope.table).values({
                scope: accountId,
                parent: ownerId,
                partition: Scope.universe.id,
                packageId: account.policy.definition.packageId,
                type: account.name,
                ancestors: [ownerId],
            });
            await this.spaces.insert(account.table).values({
                ...record,
                id: accountId,
                scope: Scope.universe.id,
                handle: `acme-${index}`,
                name: `Acme ${index}`,
                kind: "shared",
                residencyId: "eu",
            });
            this.hostnames.set(
                accountId,
                await this.serve(accountId, `notes-${index}.example.com`),
            );
        }
    }

    /** Record each space with its scope chain and an installation. */
    async #recordSpaces(
        ownerId: Identifier<"user">,
        record: { readonly createdAt: number; readonly updatedAt: number },
    ): Promise<void> {
        const accounts = [this.accountId, this.accountId, this.otherId];
        for (const [index, spaceId] of this.spaceIds.entries()) {
            const accountId = aligned(accounts, index);
            const installationId = Identifier.create("installation");
            await this.spaces.insert(space.table).values({
                ...record,
                id: spaceId,
                scope: accountId,
                name: `space-${index}`,
                stack: STACKS.space,
            });
            await this.spaces.insert(Scope.table).values({
                scope: spaceId,
                parent: accountId,
                partition: spaceId,
                packageId: space.policy.definition.packageId,
                type: space.name,
                ancestors: [accountId, ownerId],
            });
            await this.spaces.insert(installation.table).values({
                ...record,
                id: installationId,
                scope: spaceId,
                packageId: PACKAGE_ID,
                role: "application",
                name: "notes",
                selection: { kind: "release", version: "2026.9.0" },
            });
            this.installations.set(spaceId, installationId);
        }
    }

    /** Apply a stack's route declarations to a space. */
    apply(spaceId: Identifier<"space">, routes: JsonObject) {
        return Stack.apply({
            database: this.spaces,
            objects: this.#declared,
            manager: Manager.schema
                .omit({ name: true })
                .parse({ installationId: this.stack(spaceId), packageId: PACKAGE_ID }),
            as: this.#served.principal,
            scope: spaceId,
            now: Date.now(),
            document: { routes },
            claims: this.claims,
            policies: [space.policy],
            store: this.#served,
        });
    }

    /** Serve a hostname of an account as the universe does. */
    async serve(
        accountId: Identifier<"account">,
        name: string,
    ): Promise<{ readonly id: Identifier<"hostname">; readonly name: string }> {
        // keep the row in the machine's replicas
        const id = Identifier.create("hostname");
        const now = Date.now();
        const row = { id, name, scope: accountId, createdAt: now, updatedAt: now };
        await this.spaces.insert(hostname.table).values(row);

        // claim its name in the universe
        const object = { objectId: id, owner: accountId };
        const claimant = await Claimant.of(hostname, object, row, Snapshot.live(this.universe));
        await Claim.replace(this.universe, claimant);

        return { id, name };
    }

    /** Read the one stack installation of a space, creating it on first use. */
    stack(spaceId: string): string {
        const known = this.#stacks.get(spaceId) ?? Identifier.create("installation");
        this.#stacks.set(spaceId, known);

        return known;
    }

    /** Close both databases. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#spaces[Symbol.asyncDispose]();
        await this.#universe.close();
    }
}
