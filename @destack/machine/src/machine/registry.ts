import type { ObjectType, Fetch } from "@destack/object";
import type { ClientOf } from "@destack/object/client";
import { RegistryError } from "@destack/registry/error";
import registryPackage from "@destack/registry/package";
import type { Identifier } from "@destack/schema";
import type { Machine } from "./machine.ts";

/** Find the residency home of the account claiming an object, such as a package, as a machine. */
export async function findPublisherHome(
    machine: Pick<Machine, "directory" | "accounts" | "homes">,
    type: ObjectType,
    id: string,
): Promise<Identifier<"space">> {
    // find the claiming account, refusing an object none claims
    const account = await machine.directory.owner(type, id);
    if (account === undefined) {
        throw RegistryError.of("PUBLISHER_NOT_FOUND", `owning account for '${id}' not found`);
    }

    return await locateHome(machine, account);
}

/** Locate the home of an account's residency once per machine, as homes never move. */
export function locateHome(
    machine: Pick<Machine, "accounts" | "homes">,
    account: Identifier<"account">,
): Promise<Identifier<"space">> {
    // take a home located or locating already
    const known = machine.homes.get(account);
    if (known !== undefined) {
        return known;
    }

    // locate it at the universe once, forgetting a failed lookup
    const locating = machine.accounts().then((accounts) => accounts.residency.locate({ account }));
    machine.homes.set(account, locating);
    locating.catch(() => machine.homes.delete(account));

    return locating;
}

/** Call the registry in a home as a machine bound to the home, or through a caller's fetch. */
export function callRegistry(
    machine: Pick<Machine, "directory" | "registry" | "fetch" | "environment">,
    home: Identifier<"space">,
    fetch: Fetch = machine.fetch(registryPackage.id, { partition: home }),
): ClientOf<typeof registryPackage> {
    return machine.directory.client(
        registryPackage,
        home,
        machine.registry,
        fetch,
        machine.environment,
    );
}
