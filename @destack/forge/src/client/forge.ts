import { account, Space, space } from "@destack/account/object";
import { and, type DatabaseConnection, eq, type Snapshot } from "@destack/db";
import {
    Claim,
    type ClaimLookup,
    type Follow,
    installation,
    type Installation,
} from "@destack/object";
import { type Identifier, present, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import {
    packageObject,
    reference,
    referenceShape,
    repository,
    tag,
    tagShape,
} from "../object/index.ts";

/** The fields of a reference or tag row in its changes: its parent and name. */
const ChildKey = schema.looseObject({ parentId: schema.string(), name: schema.string() });

/** The forge as a space reads it: the references and tags it replicates from forge homes. */
export class Forge {
    /** The table of the references the spaces replicate. */
    readonly references = reference.table;
    /** The table of the distribution tags the spaces replicate. */
    readonly tags = tag.table;
    /** The forge rows a space's database replicates. */
    static readonly replicated = [reference, tag];
    /** The universe's name owners, which tell a repository's and a package's account. */
    readonly #claims: ClaimLookup;
    /** Read the residency home of an account, where its forge lives. */
    readonly #home: (account: Identifier<"account">) => Promise<string>;

    /** Read the forge through the universe's name owners and each account's residency home. */
    constructor(claims: ClaimLookup, home: (account: Identifier<"account">) => Promise<string>) {
        this.#claims = claims;
        this.#home = home;
    }

    /** Read the account of a repository, absent for a repository without an account. */
    async owner(id: string): Promise<Identifier<"account"> | undefined> {
        const owner = await Claim.lookup(repository, this.#claims, "id", [id]);

        return owner === undefined ? undefined : schema.identifier("account").parse(owner.scope);
    }

    /** Read the repository and name of a changed reference row. */
    reference(row: Readonly<Record<string, unknown>>) {
        const { parentId, name } = ChildKey.parse(row);

        return { repository: parentId, name };
    }

    /** Read the commit a reference resolves to in a space's replica, absent while none. */
    async commit(database: DatabaseConnection, id: string, name: string) {
        const parent = schema.identifier("repository").parse(id);
        const [followed] = await database
            .select({ commit: reference.table.commit, deletedAt: reference.table.deletedAt })
            .from(reference.table)
            .where(and(eq(reference.table.parentId, parent), eq(reference.table.name, name)));

        return followed?.deletedAt === null ? (followed.commit ?? undefined) : undefined;
    }

    /** Read the package and name of a changed tag row. */
    tag(row: Readonly<Record<string, unknown>>) {
        const { parentId, name } = ChildKey.parse(row);

        return { package: parentId, name };
    }

    /** Read the version a tag points at in a space's replica, absent while none. */
    async version(database: DatabaseConnection, packageId: string, name: string) {
        const [followed] = await database
            .select({ version: tag.table.version })
            .from(tag.table)
            .where(
                and(eq(tag.table.parentId, PackageId.parse(packageId)), eq(tag.table.name, name)),
            );

        return followed?.version;
    }

    /** Follow each served installation's selected reference and tag from its forge home. */
    follows(): Follow[] {
        return [
            installation.follow(referenceShape, {
                where: { deletedAt: { isNull: true } },
                key: async (row, view) => {
                    // follow nothing for an installation selecting no repository, or outside a space served here
                    const { selection } = row;
                    const spaceId = await this.#served(row, view);
                    if (selection.kind !== "repository" || spaceId === undefined) {
                        return undefined;
                    }

                    // follow it from the forge in the home of the space's account's residency
                    const served = present(
                        await view.read(space.table).findFirst({ where: { id: spaceId } }),
                        `space ${spaceId}`,
                    );
                    const scope = await this.#home(account.identifier(served.scope));

                    return {
                        key: { scope, repository: selection.repository, name: selection.reference },
                    };
                },
            }),
            installation.follow(tagShape, {
                where: { deletedAt: { isNull: true } },
                key: async (row, view) => {
                    // follow nothing for an installation selecting no tag, or outside a space served here
                    const { selection } = row;
                    const spaceId = await this.#served(row, view);
                    if (selection.kind !== "tag" || spaceId === undefined) {
                        return undefined;
                    }

                    // follow it from the forge in the home of the publishing account's residency
                    const ids = [row.packageId];
                    const publisher = await Claim.lookup(packageObject, this.#claims, "id", ids);
                    if (publisher === undefined) {
                        return undefined;
                    }
                    const scope = await this.#home(account.identifier(publisher.scope));

                    return { key: { scope, package: row.packageId, name: selection.tag } };
                },
            }),
        ];
    }

    /** Read the space an installation lives in when this machine serves it. */
    async #served(
        row: Pick<Installation, "scope">,
        view: Pick<Snapshot, "read">,
    ): Promise<Identifier<"space"> | undefined> {
        const spaceId = schema.identifier("space").safeParse(row.scope);

        return spaceId.success && (await Space.isServing(view, spaceId.data))
            ? spaceId.data
            : undefined;
    }
}
