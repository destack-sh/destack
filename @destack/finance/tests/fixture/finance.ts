import { type Message, t } from "@destack/locale";
import { accessRelationship, principal, Relationship } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import {
    account,
    entitlement,
    type Entitlement,
    organisation,
    Residency,
    user,
} from "@destack/account/object";
import { asc, eq, type DatabaseConnection, type Dialect, Snapshot } from "@destack/db";
import type { TestDatabase } from "@destack/db/test";
import { ObjectType } from "@destack/object";
import { ObjectStore } from "@destack/object/server";
import { nth, schema, Identifier, present, type JsonObject, type JsonValue } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { subjectContext } from "@destack/runtime/test";
import { Call, ObjectReference, Scope, Subject } from "@destack/sync";
import { v7 } from "uuid";
import {
    customer,
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
import { FinanceFixture, type FinanceFixtureOptions } from "../../src/test/index.ts";
import type { Feature, FeatureGrant } from "../../src/feature/index.ts";
import type { CatalogReference } from "../../src/catalog/reference.ts";
import type { Meter } from "@destack/observability/meter";
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
    machineSku,
    storageLimit,
    requests,
    seats,
    storage,
    stored,
    support,
    sync,
    uptime,
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

/** The bytes the Pro plan lets an account store. */
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
            uptime,
            machineSku,
        ],
    ],
    [hosting, [domains]],
] as const;

/** Finance over a test database with Acme's buyer and payer accounts and Carol's shop. */
export class Finance {
    /** The isolated database. */
    readonly test: TestDatabase;
    /** The served finance objects. */
    readonly store: ObjectStore;
    /** The served fixture. */
    readonly fixture: FinanceFixture;

    /** Create the fixture over a served finance. */
    private constructor(fixture: FinanceFixture) {
        this.fixture = fixture;
        this.test = fixture.test;
        this.store = fixture.store;
    }

    /** Open finance with Acme's buyer, Carol's shop and Dave's destack account. */
    static async open(dialect: Dialect, options: FinanceFixtureOptions = {}): Promise<Finance> {
        // serve finance with the storage and hosting packages' releases
        const fixture = await FinanceFixture.open(dialect, RELEASES, options);

        // replicate Acme with its payer
        const { test } = fixture;
        const database = test.database;
        const access = new AccessFixture(database);
        await access.replicateScope(organisation.reference(Scope.universe.id, ids.acme));
        await database.insert(organisation.table).values({
            id: ids.acme,
            scope: Scope.universe.id,
            name: "Acme",
            payer: ids.payer,
            createdAt: 0,
            updatedAt: 0,
        });

        // replicate Acme's accounts, Alice owning its buyer, and Carol's and Dave's own accounts
        const buyer = await replicateAccount(database, ids.acme, ids.buyer, "acme");
        await replicateAccount(database, ids.acme, ids.payer, "acme-billing");
        await access.bindOwner(buyer, subject("alice"), fixture.clock.now());
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
        const shop = await this.call(
            "carol",
            seller.call("create", {
                accountId: ids.shop,
                creditPrices: { EUR: "1", USD: "1", CHF: "1" },
            }),
        );
        const plan = await this.#product(t`Pro`, [
            [sync, null],
            [support, "priority"],
            [seats, SEATS],
            [calls, CALL_LIMIT],
            [storageLimit, STORAGE_LIMIT],
        ]);
        const monthly = await this.call(
            "carol",
            price.call("create", {
                parentId: plan,
                currency: "EUR",
                unitAmount: 900,
                recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
            }),
        );

        return {
            product: plan,
            seller: { scope: this.fixture.home, id: shop.id },
            monthly: { scope: this.fixture.home, id: monthly.id },
        };
    }

    /** Offer a product in Carol's shop granting features, returning its identifier. */
    async #product(
        name: Message,
        grants: readonly (readonly [Feature, JsonValue])[],
    ): Promise<string> {
        // create the product
        const created = await this.call(
            "carol",
            product.call("create", {
                accountId: ids.shop,
                name,
                description: t`An offer.`,
            }),
        );

        // grant every feature in one mutation
        const { home } = this.fixture;
        const caller = subjectContext(subject("carol"), home).access();
        await this.store.mutate(caller, {
            id: RequestId.create(this.fixture.clock.now()),
            calls: grants.map(([feature, value]) =>
                productFeature.call("create", {
                    ...productFeature.in(home),
                    parentId: created.id,
                    packageId: feature.package.id,
                    feature: feature.name,
                    value,
                }),
            ),
        });

        return created.id;
    }

    /** Store Acme's billing details as the buyer's customer, which subscribing requires. */
    async customer(): Promise<void> {
        await this.call(
            "alice",
            customer.call("create", {
                accountId: ids.buyer,
                email: "billing@acme.test",
                name: "Acme",
            }),
        );
    }

    /** Subscribe the buyer to the monthly price for one period as the provider would. */
    async subscribe(
        offer: Offer,
        period: { readonly start: number; readonly end: number },
        grants?: readonly FeatureGrant[],
    ): Promise<Subscription> {
        // create the subscription as the system
        const input = {
            ...ObjectReference.columns(
                "seller",
                seller.reference(offer.seller.scope, offer.seller.id),
            ),
            currentPeriodStart: period.start,
            currentPeriodEnd: period.end,
            providerId: `sub_${crypto.randomUUID()}`,
        };
        const created = nth(
            await this.store.mutate(this.store.serving(subscription), {
                id: RequestId.create(this.fixture.clock.now()),
                calls: [
                    subscription.call("create", {
                        ...subscription.in(this.fixture.home),
                        accountId: ids.buyer,
                        ...input,
                    }),
                ],
            }),
            0,
        );

        // bill the monthly price in it
        await this.item(created, offer.monthly, grants);

        return created;
    }

    /** Add an item billing a price to a subscription as the provider would. */
    async item(
        parent: Subscription,
        reference: Offer["monthly"],
        grants?: readonly FeatureGrant[],
    ): Promise<SubscriptionItem> {
        const offered = await Price.offer(reference, Snapshot.live(this.test.database));
        const input = {
            parentId: parent.id,
            ...ObjectReference.columns("price", price.reference(reference.scope, reference.id)),
            ...offered,
            grants: grants === undefined ? offered.grants : [...grants],
        };

        return nth(
            await this.store.mutate(this.store.serving(subscriptionItem), {
                id: RequestId.create(this.fixture.clock.now()),
                calls: [
                    subscriptionItem.call("create", {
                        ...subscriptionItem.in(this.fixture.home),
                        ...input,
                    }),
                ],
            }),
            0,
        );
    }

    /** Publish a package's next release declaring some features and meters. */
    publish(
        owner: Package,
        version: string,
        declared: readonly (Feature | Meter)[],
    ): Promise<void> {
        return this.fixture.publish({ ...owner, version }, declared);
    }

    /** Close the database. */
    close(): Promise<void> {
        return this.fixture.close();
    }

    /** Call a method as a person in the residency's home space, where finance stores its rows. */
    call<Result>(person: Person, call: Call<Result>): Promise<Result> {
        // place the call in the home, and run it on its type
        const object = present(ObjectType.find(this.store.objects, call), Call.label(call));
        const placed = { ...call, input: { ...object.in(this.fixture.home), ...call.input } };

        return this.#run(person, object, placed);
    }

    /** Send a call as a person in the home space, its input unchecked until the store parses it. */
    send(person: Person, object: ObjectType, name: string, input: JsonObject): Promise<unknown> {
        const call = object.call(name, { ...object.in(this.fixture.home), ...input });

        return this.#run(person, object, call);
    }

    /** Run a call as a person: a query, else a one-call mutation. */
    #run<Result>(person: Person, object: ObjectType, call: Call<Result>): Promise<Result> {
        const caller = subjectContext(subject(person), this.fixture.home).access();

        return object.method(call.method).mutates
            ? this.store
                  .mutate(caller, { id: RequestId.create(this.fixture.clock.now()), calls: [call] })
                  .then(([written]) => written)
            : this.store.query(caller, call);
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
            .where(eq(entitlement.table.accountId, scope))
            .orderBy(asc(entitlement.table.packageId), asc(entitlement.table.feature));
    }
}

/** Read an account's entitlement to a feature as what its kind grants, null without one. */
export function resolved(rows: readonly Entitlement[], feature: CatalogReference) {
    // find the feature's row
    const row = rows.find(
        (each) => each.packageId === feature.packageId && each.feature === feature.name,
    );

    // keep the fields its kind grants
    if (row === undefined) {
        return null;
    } else if (row.kind === "boolean") {
        return { kind: row.kind };
    } else if (row.kind === "static") {
        return { kind: row.kind, value: row.value };
    }
    const { kind, limit, usage, resetAt, state } = row;

    return { kind, limit, usage, resetAt, state };
}

/** Read a person's subject. */
export function subject(person: Person): Subject {
    return Subject.parse(principal.user.reference(Scope.universe.id, people[person]));
}

/** Replicate an account below its user or organisation, as the account package replicates it. */
async function replicateAccount(
    database: DatabaseConnection,
    scope: string,
    id: Identifier<"account">,
    handle: string,
): Promise<ObjectReference> {
    const reference = account.reference(scope, id);
    await new AccessFixture(database).replicateScope(reference);
    await database.insert(account.table).values({
        id,
        scope,
        handle,
        name: handle,
        residencyId: "eu",
        createdAt: 0,
        updatedAt: 0,
    });

    // let the servers of its residency serve it
    await database.insert(accessRelationship).values(
        Relationship.encode(
            {
                id: schema.identifier("relationship").parse(`relationship-${v7()}`),
                object: reference,
                relation: "resident",
                subject: Residency.servers("eu"),
                createdAt: 0,
                expiresAt: null,
            },
            id,
        ),
    );

    return reference;
}
