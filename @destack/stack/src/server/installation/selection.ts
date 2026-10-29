import { space } from "@destack/account/object";
import { type InstallationSelection } from "@destack/object";
import type { Snapshot } from "@destack/db";
import { type Identifier, schema } from "@destack/schema";
import type { Forge } from "./follow.ts";
import type {} from "@destack/package/import-meta";
import { StackError } from "../../error/index.ts";

/** The repository an installation selects in the account of the space. */
export const SelectedRepository = {
    /** Read the account of a selection's repository, null for none. */
    async owner(
        forge: Pick<Forge, "owner"> | null,
        selection: InstallationSelection | undefined,
    ): Promise<Identifier<"account"> | null> {
        // give no account for other selections
        if (selection?.kind !== "repository") {
            return null;
        }

        // read the account of the repository
        if (forge === null) {
            throw StackError.of(
                "REPOSITORIES_MISSING",
                "this machine does not follow any repositories",
            );
        }
        const owner = await forge.owner(selection.repository);
        if (owner === undefined) {
            throw SelectedRepository.missing(selection.repository);
        }

        return owner;
    },

    /** Refuse a selection of a repository outside the space's account. */
    async require(
        snapshot: Pick<Snapshot, "read">,
        spaceId: string,
        selection: InstallationSelection | undefined,
        owner: Identifier<"account"> | null,
    ): Promise<void> {
        // pass a selection of no repository
        if (selection?.kind !== "repository" || owner === null) {
            return;
        }

        // require the space's account
        const selected = await snapshot
            .read(space.table)
            .findFirst({ where: { id: schema.identifier("space").parse(spaceId) } });
        if (selected?.scope !== owner) {
            throw SelectedRepository.missing(selection.repository);
        }
    },

    /** Refuse a repository outside the space's account, whatever other account has it. */
    missing(id: string): Error {
        return StackError.of("REPOSITORY_NOT_FOUND", `the space's account has no repository ${id}`);
    },
};
