import { proof } from "../object/proof.ts";
import type { ObjectExtension } from "@destack/object/server";
import type { BetterAuthAuthenticator } from "../better-auth/authenticator.ts";
import type { DnsResolver } from "./dns-resolver.ts";
import type { PlatformDeclaration } from "./platform.ts";
import { Allowance } from "../object/allowance.ts";
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
import { implementPersonalAccessToken } from "./personal-access-token.ts";
import { implementUser } from "./user.ts";

/** What the universe serves the account package with. */
export interface AccountOptions {
    /** The resolver reading domains' challenge records. */
    readonly resolver: DnsResolver;
    /** The stacks spaces are born with. */
    readonly stacks: PlatformDeclaration["stacks"];
    /** The universe's sign-in, which answers the oauth adapter once the store exists. */
    readonly signIn: Promise<BetterAuthAuthenticator>;
}

/** Implement the account package in the universe's store, its oauth adapter answering sign-in. */
export function accountExtension(options: AccountOptions): ObjectExtension {
    return {
        package: accountPackage,
        objects: Object.values({
            ...accountPackage.objects,
            user: implementUser(),
            account: implementAccount(options.stacks),
            organisation: implementOrganisation(),
            member: implementMember(),
            group: implementGroup(),
            personalAccessToken: implementPersonalAccessToken(),
            client: implementClient(),
            key: implementKey(),
            oauthClient: implementOAuthClient(),
            oauthConsent: implementOAuthConsent(),
            space: implementSpace(options.stacks),
            domain: implementDomain({ resolver: options.resolver }),
            hostname: implementHostname(),
            machine: implementMachine(),
            identity: implementIdentity(),
            operation: implementOperation(),
            residency: implementResidency(),
            placement: implementPlacement(),
            claim,
            proof,
        }),
        follows: [Allowance.universe],
        serve: () => ({
            adapters: {
                oauth: async (request) => (await options.signIn).handler(request),
            },
            responseHeaders: { "Cache-Control": "no-store" },
        }),
    };
}
