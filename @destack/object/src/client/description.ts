import type { Kind } from "@destack/package";
import { present, type schema } from "@destack/schema";
import { ObjectTypeReference } from "@destack/sync";
import { Action, ACTION_KIND } from "../declare/action.ts";
import { ObjectDescription, ObjectTypePool } from "../object/description.ts";
import type { ObjectType } from "../object/object.ts";
import type { Client } from "../procedure/index.ts";
import type { PackageImplementationDescription, StoreProcedures } from "../replica/replica.ts";

/** A declaration a store's package lists, its description read by its kind's schema. */
export type PackageDeclaration<Description> = Omit<
    PackageImplementationDescription["declarations"][number],
    "description"
> & {
    /** The description, read by its kind's schema. */
    readonly description: Description;
};

/** What a store serves, as it describes itself: its packages, types and declarations. */
export class ObjectStoreDescription {
    /** The packages the store serves, with their listed types and their declarations. */
    readonly packages: readonly PackageImplementationDescription[];
    /** The object types declared so far: the ones its reader knew and the ones used since. */
    readonly pool: ObjectTypePool;

    /** The store describing a type on its first use, absent among known types alone. */
    readonly #store: Client<StoreProcedures> | undefined;

    /** Keep a store's served packages, the known types and the store describing others. */
    constructor(
        packages: readonly PackageImplementationDescription[],
        pool: ObjectTypePool,
        store?: Client<StoreProcedures>,
    ) {
        // keep the packages, the types and where the others come from
        this.packages = packages;
        this.pool = pool;
        this.#store = store;
    }

    /** List what a store serves: its packages with their types' entries and declarations. */
    static async read(
        store: Client<StoreProcedures>,
        options: {
            /** The types the reader has as code, which the store need not describe. */
            readonly known?: readonly ObjectType[];
            /** The declaration kinds to list of each package, none by default. */
            readonly kinds?: readonly Pick<Kind, "identity">[];
        } = {},
    ): Promise<ObjectStoreDescription> {
        const { known = [], kinds = [] } = options;

        // list the store's packages with their types' entries and their declarations of the kinds
        const { packages } = await store.describe({
            types: [],
            kinds: kinds.map((kind) => kind.identity),
        });

        return new ObjectStoreDescription(packages, ObjectTypePool.of(known), store);
    }

    /** Declare a listed type on its first use: a known one, else as the store describes it. */
    async type(reference: ObjectTypeReference): Promise<ObjectType> {
        // take a type the reader knows or declared before
        const { packageId, type } = reference;
        const key = ObjectTypeReference.key({ packageId, type });
        const known = this.pool.objects.get(key);
        if (known !== undefined) {
            return known;
        }

        // describe it with the types it refers to, and declare the ones the reader lacks
        const store = present(this.#store, `the store describing ${type}`);
        const { objects } = await store.describe({ types: [{ packageId, type }] });
        const unknown = objects
            .map((object) => ObjectDescription.parse(object))
            .filter((each) => !this.pool.objects.has(ObjectDescription.key(each)));
        this.pool.declare(unknown);

        return present(this.pool.objects.get(key), `the type ${type} of ${packageId}`);
    }

    /** Select a package's declarations of a kind, read by the kind's schema. */
    declarations<Description extends schema.Schema>(
        implementation: PackageImplementationDescription,
        kind: Kind<Description>,
    ): PackageDeclaration<schema.Output<Description>>[] {
        return kind.select(implementation.declarations);
    }

    /** List the actions a package declares that call a type's methods, on that type. */
    actions(implementation: PackageImplementationDescription, type: ObjectType): Action[] {
        const { packageId, type: name } = type.typeReference;

        return this.declarations(implementation, ACTION_KIND)
            .filter(
                ({ description }) =>
                    description.call?.packageId === packageId && description.call.type === name,
            )
            .map((declared) => Action.of(declared.name, declared.description, type));
    }
}
