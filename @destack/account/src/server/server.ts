import { proof } from "../object/proof.ts";
import { defineServer, type PackageServer } from "@destack/object/server";
import type { BetterAuthAuthenticator } from "../better-auth/authenticator.ts";
import type { DnsResolver } from "./dns-resolver.ts";
import type { SpaceKind, StackSource } from "../object/stack-source.ts";
import { claim } from "../object/claim.ts";
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
import { implementMember } from "./member.ts";
import { implementOAuthClient } from "./oauth-client.ts";
import { implementOAuthConsent } from "./oauth-consent.ts";
import { implementOrganisation } from "./organisation.ts";
import { implementPlacement } from "./placement.ts";
import { implementResidency } from "./residency.ts";
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
}

/** Serve the account package in the universe's store, its oauth adapter answering sign-in. */
export function accountServer(options: AccountOptions): PackageServer {
    return defineServer(accountPackage, {
        objects: [
            implementUser(),
            implementAccount(options.stacks),
            implementOrganisation(),
            implementMember(),
            implementGroup(),
            implementClient(),
            implementKey(),
            implementOAuthClient(),
            implementOAuthConsent(),
            implementSpace(options.stacks),
            implementDomain({ resolver: options.resolver }),
            implementHostname(),
            implementMachine(),
            implementIdentity(),
            implementOperation(),
            implementResidency(),
            implementPlacement(),
            claim,
            proof,
        ],
        adapters: { oauth: async (request) => (await options.signIn).handler(request) },
        responseHeaders: { "Cache-Control": "no-store" },
    });
}
