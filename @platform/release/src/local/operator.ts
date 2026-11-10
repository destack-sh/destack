import { Issuer, type TokenBinding, TokenSet } from "@destack/account/client";
import { ACCESS_TOKEN_TYPE } from "@destack/account/oauth";
import { TokenCache } from "@destack/access/token";
import type { Environment } from "@destack/environment";
import { type Fetch, PackageMount } from "@destack/object";
import { createClient } from "@destack/object/client";
import objectPackage from "@destack/object/package";
import type { PackageId } from "@destack/package";
import { Identifier } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { copyRequest } from "@destack/environment";
import { PLATFORM } from "@platform/stack";
import type { PrintedMail } from "./mail.ts";

/** The local universe's operator, signed in by the code its printed mail carries. */
export class Operator {
    /** The operator a local universe declares as its platform organization's owner. */
    static readonly LOCAL = "operator@destack.test";

    /** The issuer the operator signed in at. */
    readonly #issuer: Issuer;
    /** The client the operator signed in on. */
    readonly #clientId: Identifier<"client">;
    /** The client's latest refresh token, which each refresh rotates. */
    #refreshToken: string;
    /** The operator's tokens: the account package's, and those exchanged for others. */
    readonly #tokens = new TokenCache();
    /** The environment tokens are refreshed and exchanged at. */
    readonly #environment: Environment;

    /** Keep a signed-in operator's client and tokens. */
    private constructor(
        issuer: Issuer,
        clientId: Identifier<"client">,
        refreshToken: string,
        environment: Environment,
    ) {
        // keep the issuer, and the client's refresh token
        this.#issuer = issuer;
        this.#clientId = clientId;
        this.#refreshToken = refreshToken;
        this.#environment = environment;
    }

    /** Sign the operator in by a mailed code, accepting their pending invitations. */
    static async signIn(
        issuer: Issuer,
        mail: PrintedMail,
        environment: Environment,
    ): Promise<Operator> {
        // mail a code and redeem it, signing the operator up in a new universe
        const printed = mail.lines.length;
        await issuer.sendCode(Operator.LOCAL);
        const code = await mail.code(printed, environment.clock);
        const clientId = Identifier.create("client", environment);
        const { tokens, userId } = await issuer.redeemCode(clientId, Operator.LOCAL, code, {
            name: "Operator",
            handle: "local-operator",
            residencyId: PLATFORM.residencyId,
            timeZone: "UTC",
            locale: "en",
        });
        const operator = new Operator(issuer, clientId, TokenSet.refreshToken(tokens), environment);

        // accept each pending invitation, the platform organization's ownership among them
        const { items } = await issuer.accounts(tokens.accessToken).user.invited({ id: userId });
        const objects = createClient(
            objectPackage,
            {
                url: PackageMount.url(new URL(issuer.url).origin, objectPackage.id),
                fetch: operator.fetch(objectPackage.id),
            },
            environment,
        );
        for (const invitation of items) {
            const { object } = invitation.relationship;
            await objects.invitation.accept({
                scope: object.scope,
                requestId: RequestId.create(environment),
                id: invitation.id,
            });
        }

        return operator;
    }

    /** Read the operator's account token, refreshed once the cache stops reusing it. */
    #current(): Promise<string> {
        return this.#tokens.token("session", this.#environment.clock.now(), async () => {
            // refresh, keeping the rotated refresh token
            const tokens = await this.#issuer.refresh(this.#clientId, this.#refreshToken);
            this.#refreshToken = TokenSet.refreshToken(tokens);

            return tokens.accessToken;
        });
    }

    /** Wrap the network to call a package as the operator, in a partition when given. */
    fetch(audience: PackageId, binding: Pick<TokenBinding, "partition"> = {}): Fetch {
        return async (request) => {
            // exchange the operator's token for the package's once, reusing it while fresh
            const { partition } = binding;
            const key = JSON.stringify([audience, partition ?? null]);
            const accessToken = await this.#tokens.token(
                key,
                this.#environment.clock.now(),
                async () => {
                    const exchanged = await this.#issuer.exchange(
                        { token: await this.#current(), type: ACCESS_TOKEN_TYPE },
                        audience,
                        partition === undefined ? {} : { partition },
                    );

                    return exchanged.accessToken;
                },
            );
            const headers = new Headers(request.headers);
            headers.set("authorization", `Bearer ${accessToken}`);

            return this.#environment.network.fetch(copyRequest(request, { headers }));
        };
    }
}
