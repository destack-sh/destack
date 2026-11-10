import { FORGE_INSTALLATION } from "@destack/forge/package";
import registryPackage from "@destack/registry/package";
import { PackageName } from "@destack/package";
import { type ClientOf, createClient } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

import type { Context } from "../context/context.ts";

/** Open the forge of a package name's account, in its residency's home, as the person. */
export async function openForge(
    name: string,
    context: Context,
): Promise<{
    /** The forge's client. */
    readonly client: ClientOf<typeof registryPackage>;
    /** The residency's home serving the forge. */
    readonly home: Identifier<"space">;
    /** The account of the name's handle. */
    readonly accountId: Identifier<"account">;
}> {
    // find the account of the name's handle and its residency's home
    const native = await context.native();
    const launcher = await context.launcher();
    const accountId = await native.resolve(PackageName.split(name).handle);
    const home = await launcher.residency(accountId);

    // call the home's forge
    const endpoint = await launcher.installation(home, FORGE_INSTALLATION, registryPackage.id);

    return { client: createClient(registryPackage, endpoint, native.environment), home, accountId };
}
