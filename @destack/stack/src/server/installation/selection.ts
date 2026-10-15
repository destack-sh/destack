import { space } from "@destack/account/object";
import type { MethodContext } from "@destack/object";
import { type Identifier, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import type { InstallationSelection } from "../../declare/installation.ts";

/** Read the account owning a repository, absent for a repository no account owns. */
export type RepositoryOwner = (id: string) => Promise<Identifier<"account"> | undefined>;

/** The repository an installation selects in the account that owns the space. */
export const SelectedRepository = {
    /** Read the account owning a selection's repository, null for none. */
    async owner(
        repositories: RepositoryOwner | null,
        selection: InstallationSelection | undefined,
    ): Promise<Identifier<"account"> | null> {
        // give no account for other selections
        if (selection?.kind !== "repository") {
            return null;
        }

        // read the account owning the repository
        if (repositories === null) {
            throw new CallError("PRECONDITION_FAILED", {
                message: "this machine follows no repositories",
            });
        }
        const owner = await repositories(selection.repository);
        if (owner === undefined) {
            throw SelectedRepository.missing(selection.repository);
        }

        return owner;
    },

    /** Refuse a selection of a repository the space's account does not own. */
    async require(
        context: Pick<MethodContext, "read">,
        spaceId: string,
        selection: InstallationSelection | undefined,
        owner: Identifier<"account"> | null,
    ): Promise<void> {
        // pass a selection of no repository
        if (selection?.kind !== "repository" || owner === null) {
            return;
        }

        // require the space's account
        const selected = await context
            .read(space)
            .findFirst({ where: { id: schema.identifier("space").parse(spaceId) } });
        if (selected?.scope !== owner) {
            throw SelectedRepository.missing(selection.repository);
        }
    },

    /** Refuse a repository the space's account does not own, whoever else owns it. */
    missing(id: string): Error {
        return new CallError("NOT_FOUND", {
            message: `the space's account has no repository ${id}`,
        });
    },
};
