import type { UniverseOrigin } from "../address/address.ts";
import { implementPackage, type PackageImplementation } from "@destack/object/server";
import type { BetterAuthAuthenticator } from "../better-auth/authenticator.ts";
import type { DnsResolver } from "./dns-resolver.ts";
import type { PlatformDefinition } from "./universe.ts";
import { claim } from "../object/claim.ts";
import { stage } from "../object/stage.ts";
import accountPackage from "../package.ts";
import { implementAccount } from "./account.ts";
import { implementClient } from "./client.ts";
import { implementDomain } from "./domain.ts";
import { implementHostname } from "./hostname.ts";
import { implementGroup } from "./group.ts";
import { implementIdentity } from "./identity.ts";
import { implementOperation } from "./operation.ts";
import { implementKey } from "./key.ts";
import { implementMachine } from "./machine.ts";
import { implementApplication } from "./application.ts";
import { implementOrganization } from "./organization.ts";
import { implementPlacement } from "./placement.ts";
import { implementRegion } from "./region.ts";
import type { PackageId, PackageRelease } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { implementSpace } from "./space.ts";
import { implementUser } from "./user.ts";

/** What the universe serves the account package with. */
export interface AccountOptions {
    /** The universe, below whose domains machines' names answer. */
    readonly universe: UniverseOrigin;
    /** The resolver reading domains' challenge records. */
    readonly resolver: DnsResolver;
    /** The stacks spaces are born with. */
    readonly stacks: PlatformDefinition["stacks"];
    /** The universe's sign-in, which answers the oauth mount once the store exists. */
    readonly signIn: Promise<BetterAuthAuthenticator>;
}

/** A registry of package releases, opened by version, and the releases it ships. */
export interface ReleaseRegistry {
    /** Open a package's release. */
    readonly open: (packageId: PackageId, release: string) => Promise<BuildReader>;
    /** The releases the distribution ships, which a scope runs of a package it installs none of. */
    readonly distribution: readonly PackageRelease[];
}

/** Serve the account package in the universe's store, its oauth mount answering sign-in. */
export function accountServer(options: AccountOptions): PackageImplementation {
    return implementPackage(accountPackage, {
        objects: [
            implementUser(),
            implementAccount(options.stacks),
            implementOrganization(),
            implementGroup(),
            implementClient(),
            implementKey(),
            implementApplication(),
            implementSpace(options.stacks),
            implementDomain({ resolver: options.resolver }),
            implementHostname(),
            implementMachine(options.universe),
            implementIdentity(),
            implementOperation(),
            implementRegion(),
            implementPlacement(),
            // apply the stages an account's stack declares
            stage.implement({ declaration: {} }),
            claim,
        ],
        mounts: { oauth: async (request) => (await options.signIn).handler(request) },
        responseHeaders: { "Cache-Control": "no-store" },
    });
}
