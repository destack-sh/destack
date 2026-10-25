import { implementPackage, type PackageImplementation } from "@destack/object/server";
import type { BetterAuthAuthenticator } from "../better-auth/authenticator.ts";
import type { DnsResolver } from "./dns-resolver.ts";
import type { SpaceKind, StackSource } from "../object/stack-source.ts";
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
import { implementResidency } from "./residency.ts";
import type { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { implementSpace } from "./space.ts";
import { implementUser } from "./user.ts";

/** What the universe serves the account package with. */
export interface AccountOptions {
    /** The resolver reading domains' challenge records. */
    readonly resolver: DnsResolver;
    /** The stacks spaces are born with. */
    readonly stacks: Readonly<Record<SpaceKind, StackSource>>;
    /** The universe's sign-in, which answers the oauth adapter once the store exists. */
    readonly signIn: Promise<BetterAuthAuthenticator>;
    /** The registry of the releases the residencies' homes install. */
    readonly releases: ReleaseRegistry;
}

/** A registry of package releases, opened by version. */
export interface ReleaseRegistry {
    /** Open a package's release. */
    readonly open: (packageId: PackageId, release: string) => Promise<BuildReader>;
}

/** Serve the account package in the universe's store, its oauth adapter answering sign-in. */
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
            implementMachine(),
            implementIdentity(),
            implementOperation(),
            implementResidency(options.releases),
            implementPlacement(),
            // apply the stages an account's stack declares
            stage.implement({ declaration: {} }),
            claim,
        ],
        adapters: { oauth: async (request) => (await options.signIn).handler(request) },
        responseHeaders: { "Cache-Control": "no-store" },
    });
}
