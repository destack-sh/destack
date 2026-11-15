import { Space } from "@destack/account/object";
import { type Snapshot } from "@destack/db";
import {
    Claim,
    type ClaimLookup,
    type Follow,
    installation,
    type Installation,
} from "@destack/object";
import { Commit, type Identifier, schema } from "@destack/schema";
import { reference, referenceShape, repository, tag, tagShape } from "../object/index.ts";

/** The forge as a space reads it: the references and tags it replicates from forge homes. */
export class Forge {
    /** The table of the references the spaces replicate. */
    readonly references = reference.table;
    /** The table of the distribution tags the spaces replicate. */
    readonly tags = tag.table;
    /** The forge rows a space's database replicates. */
    static readonly replicated = [reference, tag];
    /** The universe's name owners, which tell a repository's account. */
    readonly #claims: ClaimLookup;

    /** Read the forge through the universe's name owners. */
    constructor(claims: ClaimLookup) {
        this.#claims = claims;
    }

    /** Read the account of a repository, absent for a repository without an account. */
    async owner(id: string): Promise<Identifier<"account"> | undefined> {
        const owner = await Claim.lookup(repository, this.#claims, "id", [id]);

        return owner === undefined ? undefined : schema.identifier("account").parse(owner.scope);
    }

    /** Read a followed reference in a space's replica, absent once deleted. */
    async reference(
        reader: Pick<Snapshot, "read">,
        id: string,
    ): Promise<
        | {
              readonly repository: Identifier<"repository">;
              readonly name: string;
              readonly commit: Commit | undefined;
          }
        | undefined
    > {
        const followed = await reader
            .read(reference.table)
            .findFirst({ where: { id: schema.identifier("reference").parse(id) } });

        return followed === undefined || followed.deletedAt !== null
            ? undefined
            : {
                  repository: followed.parentId,
                  name: followed.name,
                  commit: followed.commit === null ? undefined : Commit.parse(followed.commit),
              };
    }

    /** Read the version a followed tag points at in a space's replica, absent while none. */
    async version(reader: Pick<Snapshot, "read">, id: string): Promise<string | undefined> {
        const followed = await reader
            .read(tag.table)
            .findFirst({ where: { id: schema.identifier("tag").parse(id) } });

        return followed?.version;
    }

    /** Follow each served installation's followed reference and tag from its home. */
    follows(): Follow[] {
        return [
            referenceShape.per(installation, {
                where: { deletedAt: { isNull: true }, followsType: reference.name },
                key: async (row, view) => {
                    const followed = await Forge.#followed(row, view);

                    return followed === undefined
                        ? undefined
                        : {
                              key: {
                                  scope: followed.scope,
                                  id: schema.identifier("reference").parse(followed.id),
                              },
                          };
                },
            }),
            tagShape.per(installation, {
                where: { deletedAt: { isNull: true }, followsType: tag.name },
                key: async (row, view) => {
                    const followed = await Forge.#followed(row, view);

                    return followed === undefined
                        ? undefined
                        : {
                              key: {
                                  scope: followed.scope,
                                  id: schema.identifier("tag").parse(followed.id),
                              },
                          };
                },
            }),
        ];
    }

    /** Read the home and id of the row an installation follows, none outside a served space. */
    static async #followed(
        row: Pick<Installation, "scope" | "followsScope" | "followsId">,
        view: Pick<Snapshot, "read">,
    ): Promise<{ readonly scope: string; readonly id: string } | undefined> {
        // follow nothing outside a space served here
        const spaceId = schema.identifier("space").safeParse(row.scope);
        const isServed = spaceId.success && (await Space.isServing(view, spaceId.data));
        if (!isServed || row.followsScope === null || row.followsId === null) {
            return undefined;
        }

        return { scope: row.followsScope, id: row.followsId };
    }
}
