import { accessRelationship, principal, Relationship } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account, organisation, user } from "@destack/account/object";
import { asc, eq, type DatabaseConnection, type Dialect } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import type { CallableName, CallOutput, ObjectType } from "@destack/object";
import { ObjectServer } from "@destack/object/server";
import {
    aligned,
    found,
    schema,
    type Identifier,
    type JsonObject,
    type JsonValue,
} from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { reconciliation, subjectContext, testCallKey } from "@destack/service/test";
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
import { serveFinance } from "../../src/server/index.ts";
import { financeService } from "../../src/service/index.ts";
import { financeDatabase } from "../../src/stack/index.ts";
import type { Feature } from "../../src/feature/index.ts";
import type { Package } from "@destack/package";
import { release } from "./release.ts";
import {
    calls,
    domains,
    hosting,
    quota,
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

/** The seats the lifetime pack grants. */
export const PACK_SEATS = 10;

/** The API calls the Pro plan grants each period. */
export const CALL_LIMIT = 1000;

/** The bytes the Pro plan lets an account keep. */
export const QUOTA_LIMIT = 1_000_000_000;

/** The shop's seller and its prices of the Pro plan, as qualified references. */
export interface Offer {
    /** The plan's product. */
    readonly product: string;
    /** The shop's seller. */
    readonly seller: { readonly scope: string; readonly id: string };
    /** The plan's monthly price. */
    readonly monthly: { readonly scope: string; readonly id: string };
    /** The lifetime pack's one-time price. */
    readonly once: { readonly scope: string; readonly id: string };
}

/** The finance service over a migrated test database: Acme's buyer and payer accounts, and Carol's shop. */
export class Finance {
    /** The isolated database. */
    readonly test: TestDatabase;
    /** The served finance objects. */
    readonly server: ObjectServer<ReturnType<typeof serveFinance>>;
    /** The latest release of each package, by identifier. */
    readonly #releases = new Map([
        [storage.id, release(storage, [requests, stored, sync, support, seats, calls, quota])],
        [hosting.id, release(hosting, [domains])],
    ]);

    /** Serve the finance objects over a database, opening the storage and hosting packages' releases. */
    private constructor(test: TestDatabase) {
        this.test = test;
        this.server = new ObjectServer({
            objects: serveFinance({ open: (packageId) => found(this.#releases, packageId) }),
            policies: [account, organisation],
            database: test.database,
            callKey: testCallKey,
            origin: { package: financeService.package, service: financeService.name },
        });
    }

    /** Open the service with Alice owning Acme's buyer account, Bob viewing it, Carol owning her shop, and Dave the destack account. */
    static async open(dialect: Dialect): Promise<Finance> {
        // copy Acme with its payer, and Carol's and Dave's users
        const test = await TestDatabase.create(dialect, financeDatabase, { isMigrated: true });
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
        await copies.copyScope(user.reference(Scope.universe.id, people.carol));
        await copies.copyScope(user.reference(Scope.universe.id, people.dave));

        // copy the accounts, and let Alice, Carol and Dave own theirs
        const buyer = await copyAccount(database, ids.acme, ids.buyer, "acme");
        await copyAccount(database, ids.acme, ids.payer, "acme-billing");
        const shop = await copyAccount(database, people.carol, ids.shop, "shop");
        await copies.copyOwner(buyer, subject("alice"));
        const platform = await copyAccount(database, people.dave, ids.destack, "destack");
        await copies.copyOwner(shop, subject("carol"));
        await copies.copyOwner(platform, subject("dave"));

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

        return new Finance(test);
    }

    /** Offer Carol's Pro plan monthly, granting every feature, and her lifetime pack once, granting the unmetered ones. */
    async offer(): Promise<Offer> {
        // onboard the shop and offer the plan monthly
        const shop = await this.call("carol", seller, "create", ids.shop, { country: "AT" });
        const plan = await this.#product("Pro", [
            [sync, null],
            [support, "priority"],
            [seats, SEATS],
            [calls, CALL_LIMIT],
            [quota, QUOTA_LIMIT],
        ]);
        const monthly = await this.call("carol", price, "create", ids.shop, {
            parentId: plan,
            currency: "EUR",
            unitAmount: 900,
            type: "recurring",
            recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
        });

        // offer a lifetime pack once, without metered features
        const pack = await this.#product("Lifetime", [
            [sync, null],
            [support, "community"],
            [seats, PACK_SEATS],
        ]);
        const once = await this.call("carol", price, "create", ids.shop, {
            parentId: pack,
            currency: "EUR",
            unitAmount: 9900,
            type: "one_time",
        });

        return {
            product: plan,
            seller: { scope: ids.shop, id: shop.id },
            monthly: { scope: ids.shop, id: monthly.id },
            once: { scope: ids.shop, id: once.id },
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
        const snapshot = await Price.snapshot(this.test.database, reference);
        const input = {
            parentId: parent.id,
            price: reference,
            ...snapshot,
            grants: grants === undefined ? snapshot.grants : [...grants],
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

    /** Close the database. */
    close(): Promise<void> {
        return this.test.close();
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

    /** Derive every customer's entitlements, as the controller does after each change. */
    async entitle(): Promise<void> {
        const controller = this.server.controllers().find((each) => each.name === "customer");
        if (controller === undefined) {
            throw new TypeError("the customer controller is missing");
        }
        for (const key of await controller.list()) {
            await controller.reconcile(key, reconciliation());
        }
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
