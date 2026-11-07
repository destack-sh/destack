import type { Environment } from "@destack/environment";
import { FORGE_INSTALLATION } from "@destack/forge/package";
import registryPackage from "@destack/registry/package";
import { type Package, type PackageId, PackageName } from "@destack/package";
import type { Identifier } from "@destack/schema";
import { type ClientOf, createClient } from "@destack/object/client";
import { RequestId } from "@destack/sync";

import type { Context } from "../context/context.ts";

/** The forge of a package's account, in its residency's home, as the person signed in. */
export class Forge {
    /** The account of the package's handle. */
    readonly accountId: Identifier<"account">;
    /** The residency's home serving the forge. */
    readonly home: Identifier<"space">;
    /** The forge's client. */
    readonly client: ClientOf<typeof registryPackage>;
    /** The environment the forge's requests are drawn from. */
    readonly environment: Environment;

    /** Take a forge's account, home, client and environment. */
    constructor(
        accountId: Identifier<"account">,
        home: Identifier<"space">,
        client: ClientOf<typeof registryPackage>,
        environment: Environment,
    ) {
        // set the account, home, client and environment
        this.accountId = accountId;
        this.home = home;
        this.client = client;
        this.environment = environment;
    }

    /** Open the forge of a package name's account, in its residency's home, as the person. */
    static async open(name: string, context: Context): Promise<Forge> {
        // find the account of the name's handle and its residency's home
        const client = await context.native();
        const person = await context.person();
        const accountId = await client.resolve(PackageName.split(name).handle);
        const home = await person.residency(accountId);

        // call the home's forge
        const endpoint = await person.installation(home, FORGE_INSTALLATION, registryPackage.id);
        const forge = createClient(registryPackage, endpoint, client.environment);

        return new Forge(accountId, home, forge, client.environment);
    }

    /** Create a package in the forge, once per identifier, as the forge's create is idempotent. */
    async createPackage(described: Pick<Package, "id" | "name">): Promise<void> {
        await this.client.package.create({
            spaceId: this.home,
            requestId: RequestId.create(this.environment),
            id: described.id,
            accountId: this.accountId,
            name: PackageName.split(described.name).local,
        });
    }

    /** Point a package's distribution tag at a version, creating the tag once, else moving it. */
    async point(packageId: PackageId, name: string, version: string) {
        // find the package's tag of the name
        const tags = await this.client.tag.list({
            spaceId: this.home,
            where: { parentId: packageId, name },
            limit: 1,
        });
        const [found] = tags.items;
        const requestId = RequestId.create(this.environment);

        // create it, else move it at its revision
        return found === undefined
            ? await this.client.tag.create({
                  spaceId: this.home,
                  requestId,
                  parentId: packageId,
                  name,
                  version,
              })
            : await this.client.tag.update({
                  spaceId: this.home,
                  requestId,
                  id: found.id,
                  version,
                  revision: found.revision,
              });
    }

    /** Resolve a package of the forge and its release, latest or a version. */
    async resolve(name: string, release: string) {
        const found = await this.client.package.resolve({
            spaceId: this.home,
            name: PackageName.parse(name),
        });
        const resolved = await this.client.release.resolve({
            spaceId: this.home,
            parentId: found.id,
            release,
        });

        return { packageId: found.id, release: resolved };
    }
}
