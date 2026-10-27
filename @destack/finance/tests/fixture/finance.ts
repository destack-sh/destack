import { DeclarationReference } from "@destack/package/declare";
import { type Message, t } from "@destack/locale";

import { entitlement, type Entitlement } from "@destack/account/object";
import { asc, eq, type DatabaseConnection, Snapshot } from "@destack/db";
import { ObjectType } from "@destack/object";
import { ObjectStore } from "@destack/object/server";
import {
    nth,
    Identifier,
    present,
    type JsonObject,
    type JsonValue,
    Text,
    Uuid,
} from "@destack/schema";
import { RequestId, Call, ObjectReference } from "@destack/sync";
import { subjectContext } from "@destack/object/test";
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
    type Resolution,
} from "../../src/object/index.ts";
import {
    FinanceFixture,
    type FinanceFixtureOptions,
    ids,
    type Person,
} from "../../src/test/index.ts";

export { ids, type Person, spaces } from "../../src/test/index.ts";
import type { Feature, FeatureGrant } from "../../src/feature/index.ts";
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

/** Finance in the residency's home with Acme's buyer and payer accounts and Carol's shop. */
export class Finance {
    /** The served finance objects. */
    readonly store: ObjectStore;
    /** The served kit. */
    readonly fixture: FinanceFixture;

    /** Keep a served kit. */
    private constructor(fixture: FinanceFixture) {
        this.fixture = fixture;
        this.store = fixture.store;
    }

    /** Open finance with the storage and hosting packages' releases. */
    static async open(options: FinanceFixtureOptions = {}): Promise<Finance> {
        return new Finance(await FinanceFixture.open(RELEASES, options));
    }

    /** The home's database. */
    get database(): DatabaseConnection {
        return this.fixture.database;
    }

    /** The platform's account, which publishes the platform's packages. */
    get platform(): Identifier<"account"> {
        return this.fixture.platform;
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
        const caller = subjectContext(this.fixture.subject("carol"), home, {
            environment: this.fixture.environment,
        }).access();
        await this.store.mutate(caller, {
            id: RequestId.create(this.fixture.environment),
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
            providerId: `sub_${Uuid.v4(this.fixture.environment.random)}`,
        };
        const created = nth(
            await this.store.mutate(this.store.as(subscription), {
                id: RequestId.create(this.fixture.environment),
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
        const offered = await Price.offer(reference, Snapshot.live(this.database));
        const input = {
            parentId: parent.id,
            ...ObjectReference.columns("price", price.reference(reference.scope, reference.id)),
            ...offered,
            grants: grants === undefined ? offered.grants : [...grants],
        };

        return nth(
            await this.store.mutate(this.store.as(subscriptionItem), {
                id: RequestId.create(this.fixture.environment),
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
        const caller = subjectContext(this.fixture.subject(person), this.fixture.home, {
            environment: this.fixture.environment,
        }).access();

        return object.method(call.method).mutates
            ? this.store
                  .mutate(caller, { id: RequestId.create(this.fixture.environment), calls: [call] })
                  .then(([written]) => written)
            : this.store.query(caller, call);
    }

    /** Derive every account's entitlements, as the controller does after each change. */
    entitle(): Promise<void> {
        return this.fixture.entitle();
    }

    /** Resolve each feature an account's sources grant now, with its metered usage. */
    async resolutions(scope: Identifier<"account">): Promise<Resolution[]> {
        // order them as the entitlements are, by package and feature
        const resolutions = await this.fixture.resolutions(scope);

        return resolutions.toSorted(
            (left, right) =>
                Text.compare(left.packageId, right.packageId) ||
                Text.compare(left.feature, right.feature),
        );
    }

    /** Read an account's entitlements in a stable order. */
    async entitlements(scope: Identifier<"account">): Promise<Entitlement[]> {
        return await this.database
            .select()
            .from(entitlement.table)
            .where(eq(entitlement.table.scope, scope))
            .orderBy(asc(entitlement.table.packageId), asc(entitlement.table.feature));
    }
}

/** Read an account's resolution of a feature as what its kind grants, null without one. */
export function resolved(resolutions: readonly Resolution[], feature: DeclarationReference) {
    // find the feature's resolution
    const found = resolutions.find(
        (each) => each.packageId === feature.packageId && each.feature === feature.name,
    );

    // keep the fields its kind grants
    if (found === undefined) {
        return null;
    } else if (found.kind === "boolean") {
        return { kind: found.kind };
    } else if (found.kind === "static") {
        return { kind: found.kind, value: found.value };
    }
    const { kind, limit, usage, resetAt, state } = found;

    return { kind, limit, usage, resetAt, state };
}
