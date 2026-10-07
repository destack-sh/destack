import { accessRelationship, principal, Relationship } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account, organisation, user } from "@destack/account/object";
import { asc, eq, type DatabaseConnection, type Dialect } from "@destack/db";
import type { TestDatabase } from "@destack/db/test";
import type { CallableName, CallOutput, ObjectType } from "@destack/object";
import { ObjectServer } from "@destack/object/server";
import { aligned, schema, type Identifier, type JsonObject, type JsonValue } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { subjectContext } from "@destack/service/test";
import { Scope, Subject, type ObjectReference } from "@destack/sync";
import { v7 } from "uuid";
import {
    customer,
    entitlement,
    type Entitlement,
    type FeatureGrant,
    price,
    Price,
    product,
    productFeature,
    seller,
    subscription,
    type Subscription,
    subscriptionItem,
    type SubscriptionItem,
} from "../../src/object/index.ts";
import type { serveFinance } from "../../src/server/index.ts";
import { FinanceFixture, type FinanceFixtureOptions } from "../../src/test/index.ts";
import type { Feature } from "../../src/feature/index.ts";
import type { Meter } from "../../src/meter/index.ts";
import type { Package } from "@destack/package";
import {
    archived,
    calls,
    capacity,
    disk,
    diskLimit,
    diskSku,
    domains,
    hosting,
    storageLimit,
    requests,
    seats,
    storage,
    stored,
    support,
    sync,
} from "./storage.ts";

/** The people the scenarios act as. */
export const people = {
    alice: user.identifier("user-019f5530-8000-7000-8000-000000000201"),
    bob: user.identifier("user-019f5530-8000-7000-8000-000000000202"),
    carol: user.identifier("user-019f5530-8000-7000-8000-000000000203"),
    eve: user.identifier("user-019f5530-8000-7000-8000-000000000204"),
    dave: user.identifier("user-019f5530-8000-7000-8000-000000000205"),
};

/** A person the scenarios act as. */
export type Person = keyof typeof people;

/** The organisation and accounts the scenarios bill. */
export const ids = {
    acme: organisation.identifier("organisation-019f5530-8000-7000-8000-000000000301"),
    buyer: account.identifier("account-019f5530-8000-7000-8000-000000000302"),
    payer: account.identifier("account-019f5530-8000-7000-8000-000000000303"),
    shop: account.identifier("account-019f5530-8000-7000-8000-000000000304"),
    destack: account.identifier("account-019f5530-8000-7000-8000-000000000305"),
};

/** The seats the Pro plan grants. */
export const SEATS = 5;

/** The API calls the Pro plan grants each period. */
export const CALL_LIMIT = 1000;

/** The bytes the Pro plan lets an account keep. */
export const STORAGE_LIMIT = 1_000_000_000;

/** The shop's seller and its prices of the Pro plan, as qualified references. */
export interface Offer {
    /** The plan's product. */
    readonly product: string;
    /** The shop's seller. */
    readonly seller: { readonly scope: string; readonly id: string };
    /** The plan's monthly price. */
    readonly monthly: { readonly scope: string; readonly id: string };
}

/** The storage and hosting packages' releases, with the features, meters and SKUs each declares. */
const RELEASES = [
    [
        storage,
        [
            requests,
            stored,
            archived,
            disk,
            sync,
            support,
            seats,
            calls,
            storageLimit,
            capacity,
            diskLimit,
            diskSku,
        ],
    ],
    [hosting, [domains]],
] as const;

/** The finance service over a migrated test database: Acme's buyer and payer accounts, and Carol's shop. */
export class Finance {
    /** The isolated database. */
    readonly test: TestDatabase;
    /** The served finance objects. */
    readonly server: ObjectServer<ReturnType<typeof serveFinance>>;
    /** The served fixture. */
    readonly fixture: FinanceFixture;

    /** Keep the served fixture. */
    private constructor(fixture: FinanceFixture) {
        this.fixture = fixture;
        this.test = fixture.test;
        this.server = fixture.server;
    }

    /** Open the service with Alice owning Acme's buyer account, Bob viewing it, Carol owning her shop, and Dave the destack account, billing through a provider when given. */
    static async open(dialect: Dialect, options: FinanceFixtureOptions = {}): Promise<Finance> {
        // serve finance with the storage and hosting packages' releases
        const fixture = await FinanceFixture.open(dialect, RELEASES, options);

        // copy Acme with its payer
        const { test } = fixture;
        const database = test.database;
        const copies = new AccessFixture(database);
        await copies.copyScope(organisation.reference(Scope.universe.id, ids.acme));
        await database.insert(organisation.table).values({
            id: ids.acme,
            scope: Scope.universe.id,
            name: "Acme",
            residencyId: "eu",
            payer: ids.payer,
            createdAt: 0,
            updatedAt: 0,
        });

        // copy Acme's accounts, Alice owning its buyer, and Carol's and Dave's own accounts
        const buyer = await copyAccount(database, ids.acme, ids.buyer, "acme");
        await copyAccount(database, ids.acme, ids.payer, "acme-billing");
        await copies.copyOwner(buyer, subject("alice"));
        await fixture.account(ids.shop, "shop", people.carol);
        await fixture.account(ids.destack, "destack", people.dave);

        // let Bob view the buyer's account
        await database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: schema.identifier("relationship").parse(`relationship-${v7()}`),
                    object: buyer,
                    relation: "viewer",
                    subject: subject("bob"),
                    createdAt: 0,
                    expiresAt: null,
                },
                ids.buyer,
            ),
        );

        return new Finance(fixture);
    }

    /** Offer Carol's Pro plan monthly, granting every feature. */
    async offer(): Promise<Offer> {
        // onboard the shop and offer the plan monthly
        const shop = await this.call("carol", seller, "create", ids.shop, {
            country: "AT",
            creditPrices: { EUR: "1", USD: "1", CHF: "1" },
        });
        const plan = await this.#product("Pro", [
            [sync, null],
            [support, "priority"],
            [seats, SEATS],
            [calls, CALL_LIMIT],
            [storageLimit, STORAGE_LIMIT],
        ]);
        const monthly = await this.call("carol", price, "create", ids.shop, {
            parentId: plan,
            currency: "EUR",
            unitAmount: 900,
            recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
        });

        return {
            product: plan,
            seller: { scope: ids.shop, id: shop.id },
            monthly: { scope: ids.shop, id: monthly.id },
        };
    }

    /** Offer a product in Carol's shop granting features with their values, returning its identifier. */
    async #product(
        name: string,
        grants: readonly (readonly [Feature, JsonValue])[],
    ): Promise<string> {
        // create the product
        const created = await this.call("carol", product, "create", ids.shop, {
            name,
            description: `The ${name} offer.`,
        });

        // grant each feature
        for (const [feature, value] of grants) {
            await this.call("carol", productFeature, "create", ids.shop, {
                parentId: created.id,
                packageId: feature.package.id,
                feature: feature.name,
                value,
            });
        }

        return created.id;
    }

    /** Keep Acme's billing details as the buyer's customer, which subscribing and buying require. */
    async customer(): Promise<void> {
        await this.call("alice", customer, "create", ids.buyer, {
            email: "billing@acme.test",
            name: "Acme",
        });
    }

    /** Subscribe the buyer to the monthly price as the provider would, for one billing period, its grants replaced when given. */
    async subscribe(
        offer: Offer,
        period: { readonly start: number; readonly end: number },
        grants?: readonly FeatureGrant[],
    ): Promise<Subscription> {
        // create the subscription as the system
        const input = {
            seller: offer.seller,
            currentPeriodStart: period.start,
            currentPeriodEnd: period.end,
            providerId: `sub_${crypto.randomUUID()}`,
        };
        const created = aligned(
            await this.server.executeAsSystem(
                subscription,
                "create",
                [{ scope: ids.buyer, input }],
                Date.now(),
            ),
            0,
        );

        // bill the monthly price in it
        await this.item(created, offer.monthly, grants);

        return created;
    }

    /** Add an item billing a price to a subscription as the provider would, its grants replaced when given. */
    async item(
        parent: Subscription,
        reference: Offer["monthly"],
        grants?: readonly FeatureGrant[],
    ): Promise<SubscriptionItem> {
        const offered = await Price.offer(this.test.database, reference);
        const input = {
            parentId: parent.id,
            price: reference,
            ...offered,
            grants: grants === undefined ? offered.grants : [...grants],
        };

        return aligned(
            await this.server.executeAsSystem(
                subscriptionItem,
                "create",
                [{ scope: ids.buyer, input }],
                Date.now(),
            ),
            0,
        );
    }

    /** Publish a later release of a package declaring some features and meters, which becomes its latest. */
    publish(owner: Package, version: string, declared: readonly (Feature | Meter)[]): void {
        this.fixture.publish({ ...owner, version }, declared);
    }

    /** Close the database. */
    close(): Promise<void> {
        return this.fixture.close();
    }

    /** Call a method as a person in an account. */
    async call<Type extends ObjectType, Name extends CallableName<Type>>(
        person: Person,
        object: Type,
        name: Name,
        scope: string,
        input: JsonObject,
    ): Promise<CallOutput<Type, Name>> {
        // place the call in the account through the object's route field
        const field = object.route.field;
        if (field === undefined) {
            throw new TypeError("a scoped call needs an object routed by a field");
        }

        return await this.server.call(
            object,
            name,
            { [field]: scope, requestId: RequestId.create(), ...input },
            subjectContext(subject(person), scope),
        );
    }

    /** Derive every account's entitlements, as the controller does after each change. */
    entitle(): Promise<void> {
        return this.fixture.entitle();
    }

    /** Run an object type's controller over every pending key once. */
    control(name: string): Promise<void> {
        return this.fixture.control(name);
    }

    /** Read an account's entitlements in a stable order. */
    async entitlements(scope: Identifier<"account">): Promise<Entitlement[]> {
        return await this.test.database
            .select()
            .from(entitlement.table)
            .where(eq(entitlement.table.scope, scope))
            .orderBy(asc(entitlement.table.source), asc(entitlement.table.feature));
    }
}

/** Read a person's subject. */
export function subject(person: Person): Subject {
    return Subject.parse(principal.user.reference(Scope.universe.id, people[person]));
}

/** Copy an account below its user or organisation, as the account service replicates it. */
async function copyAccount(
    database: DatabaseConnection,
    scope: string,
    id: Identifier<"account">,
    handle: string,
): Promise<ObjectReference> {
    const reference = account.reference(scope, id);
    await new AccessFixture(database).copyScope(reference);
    await database.insert(account.table).values({
        id,
        scope,
        handle,
        name: handle,
        residencyId: "eu",
        createdAt: 0,
        updatedAt: 0,
    });

    return reference;
}
