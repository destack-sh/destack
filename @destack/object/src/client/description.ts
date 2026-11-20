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
    /** The object types the store describes, beside the ones its reader knew. */
    readonly pool: ObjectTypePool;

    /** Keep a store's served packages and the types they list. */
    constructor(packages: readonly PackageImplementationDescription[], pool: ObjectTypePool) {
        this.packages = packages;
        this.pool = pool;
    }

    /** Describe a store in one call, declaring the types it serves beside some the reader knows. */
    static async read(
        store: Client<StoreProcedures>,
        known: readonly ObjectType[] = [],
    ): Promise<ObjectStoreDescription> {
        // describe the store's packages and types
        const { objects, packages } = await store.describe({});

        // declare the types the reader does not know
        const pool = ObjectTypePool.of(known);
        const unknown = objects
            .map((object) => ObjectDescription.parse(object))
            .filter((description) => !pool.objects.has(ObjectDescription.key(description)));
        pool.declare(unknown);

        return new ObjectStoreDescription(packages, pool);
    }

    /** Find a described type by its reference, refusing one the store does not describe. */
    type(reference: ObjectTypeReference): ObjectType {
        return present(
            this.pool.objects.get(ObjectTypeReference.key(reference)),
            `the type ${reference.type} of ${reference.packageId}`,
        );
    }

    /** List the types a package lists. */
    types(implementation: PackageImplementationDescription): ObjectType[] {
        return implementation.types.map((reference) => this.type(reference));
    }

    /** Select a package's declarations of a kind, read by the kind's schema. */
    declarations<Description extends schema.Schema>(
        implementation: PackageImplementationDescription,
        kind: Kind<Description>,
    ): PackageDeclaration<schema.Output<Description>>[] {
        return kind.select(implementation.declarations);
    }

    /** List the actions a package declares, each call on the type it calls. */
    actions(implementation: PackageImplementationDescription): Action[] {
        return this.declarations(implementation, ACTION_KIND).map(({ name, description }) => {
            const { call } = description;
            const type =
                call === undefined
                    ? undefined
                    : this.type({ packageId: call.packageId, type: call.type });

            return Action.of(name, description, type);
        });
    }
}
