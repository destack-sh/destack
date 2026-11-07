import type { DeclarationKind } from "@destack/package";
import { present, type schema } from "@destack/schema";
import { ObjectTypeReference } from "@destack/sync";
import { Command, COMMAND_KIND } from "../declare/command.ts";
import { ObjectDescription, ObjectTypePool } from "../object/description.ts";
import type { ObjectType } from "../object/object.ts";
import type { Client } from "../procedure/index.ts";
import type { InstallationDescription, StoreProcedures } from "../replica/replica.ts";

/** A declaration an installation declares, its description read by its kind's schema. */
export type InstallationDeclaration<Description> = Omit<
    InstallationDescription["declarations"][number],
    "description"
> & {
    /** The description, read by its kind's schema. */
    readonly description: Description;
};

/** What a store serves, as it describes itself: its installations, types and declarations. */
export class ObjectStoreDescription {
    /** The installations the store runs, with their listed types and their declarations. */
    readonly installations: readonly InstallationDescription[];
    /** The object types the store describes, beside the ones its reader knew. */
    readonly pool: ObjectTypePool;

    /** Keep a store's installations and the types they list. */
    constructor(installations: readonly InstallationDescription[], pool: ObjectTypePool) {
        this.installations = installations;
        this.pool = pool;
    }

    /** Describe a store in one call, declaring the types it serves beside some the reader knows. */
    static async read(
        store: Client<StoreProcedures>,
        known: readonly ObjectType[] = [],
    ): Promise<ObjectStoreDescription> {
        // describe the store's installations and types
        const { objects, installations } = await store.describe({});

        // declare the types the reader does not know
        const pool = ObjectTypePool.of(known);
        const unknown = objects
            .map((object) => ObjectDescription.parse(object))
            .filter((described) => !pool.objects.has(ObjectDescription.key(described)));
        pool.declare(unknown);

        return new ObjectStoreDescription(installations, pool);
    }

    /** Find a described type by its reference, refusing one the store does not describe. */
    type(reference: ObjectTypeReference): ObjectType {
        return present(
            this.pool.objects.get(ObjectTypeReference.key(reference)),
            `the type ${reference.type} of ${reference.packageId}`,
        );
    }

    /** List the types an installation lists. */
    types(installation: InstallationDescription): ObjectType[] {
        return installation.types.map((reference) => this.type(reference));
    }

    /** Select an installation's declarations of a kind, read by the kind's schema. */
    declarations<Description extends schema.Schema>(
        installation: InstallationDescription,
        kind: DeclarationKind<Description>,
    ): InstallationDeclaration<schema.Output<Description>>[] {
        return kind.select(installation.declarations);
    }

    /** List the commands an installation declares, each on the type it calls. */
    commands(installation: InstallationDescription): Command[] {
        return this.declarations(installation, COMMAND_KIND).map(({ name, description }) =>
            Command.of(
                name,
                description,
                this.type({ packageId: description.packageId, type: description.type }),
            ),
        );
    }
}
