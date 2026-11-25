import type { ObjectType, Fetch } from "@destack/object";
import type { ClientOf } from "@destack/object/client";
import { RegistryError } from "@destack/registry/error";
import registryPackage from "@destack/registry/package";
import type { Identifier } from "@destack/schema";
import type { Machine } from "./machine.ts";
import { Account } from "@destack/account/object";

/** Find the home of the account claiming an object, such as a package, as a machine. */
export async function findPublisherHome(
    machine: Pick<Machine, "directory">,
    type: ObjectType,
    id: string,
): Promise<Identifier<"space">> {
    // find the claiming account, refusing an object none claims
    const account = await machine.directory.owner(type, id);
    if (account === undefined) {
        throw RegistryError.of("PUBLISHER_NOT_FOUND", `owning account for '${id}' not found`);
    }

    return Account.home(account);
}

/** Call the registry in a home as a machine bound to the home, or through a caller's fetch. */
export function callRegistry(
    machine: Pick<Machine, "directory" | "fetch" | "environment">,
    home: Identifier<"space">,
    fetch: Fetch = machine.fetch(registryPackage.id, { partition: home }),
): ClientOf<typeof registryPackage> {
    return machine.directory.client(registryPackage, home, fetch, machine.environment);
}
