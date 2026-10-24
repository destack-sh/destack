import { principal } from "@destack/access";
import { TokenVerifier } from "@destack/access/token";
import { Identity } from "@destack/account/object";
import type { Snapshot } from "@destack/db";
import type { Environment } from "@destack/environment";
import { Jwks, Jws } from "@destack/identity";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { RelayError } from "../error/index.ts";
import relayPackage from "../package.ts";
import type { TunnelAttachment } from "./tunnel.ts";

/** The lifetime of tunnel tokens: ten minutes, Kubernetes' shortest bound service account token. */
export const TUNNEL_TOKEN_LIFETIME_MILLISECONDS = 10 * 60_000;

/** The universe's tunnel tokens, which machines open and renew their tunnels with. */
export class TunnelToken {
    /** Verify the universe's tokens for the relay against the replicated keys. */
    readonly #verifier: TokenVerifier;
    /** The replica of the universe's identities, whose keys sign the tokens. */
    readonly #replica: Pick<Snapshot, "read">;
    /** The WebCrypto reading the universe's keys. */
    readonly #crypto: Environment["crypto"];

    /** Verify the universe's tokens by its keys in a replica, following their rotations. */
    constructor(
        issuer: string,
        replica: Pick<Snapshot, "read">,
        environment: Pick<Environment, "clock" | "crypto" | "network">,
    ) {
        this.#replica = replica;
        this.#crypto = environment.crypto;
        this.#verifier = new TokenVerifier({
            authority: { kind: "universe" },
            issuer,
            audience: relayPackage.id,
            lifetime: TUNNEL_TOKEN_LIFETIME_MILLISECONDS,
            environment,
            // read the current keys at each verify
            keys: async (header) => {
                const { crypto } = environment;
                const keys = await Identity.keys(replica, Scope.universe.id, crypto);

                return Jwks.local(keys, crypto)(header);
            },
        });
    }

    /** Verify a machine's token, answering the machine, its token's lapse and its signing key. */
    async verify(token: string, now: number): Promise<TunnelAttachment> {
        // require a machine's token
        const { caller, credential } = await this.#verifier.verify(token, now);
        const { subject } = caller;
        if (!principal.machine.is(subject)) {
            throw RelayError.of("MACHINE_REQUIRED", "only machines can open tunnels");
        }

        return {
            machineId: schema.identifier("machine").parse(subject.id),
            lapsesAt: credential.expiresAt,
            kid: schema.string().min(1).parse(Jws.header(token).kid),
        };
    }

    /** Report whether the universe's identities still list a signing key. */
    async isStanding(kid: string): Promise<boolean> {
        const { keys } = await Identity.keys(this.#replica, Scope.universe.id, this.#crypto);

        return keys.some((key) => key.kid === kid);
    }
}
