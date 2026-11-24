import { CONTAINED, principal } from "@destack/access";
import { TokenVerifier } from "@destack/access/token";
import { Identity, residency, type ResidencyCode } from "@destack/account/object";
import type { Snapshot } from "@destack/db/database";
import type { Environment } from "@destack/environment";
import { Jws } from "@destack/identity";
import { present, schema } from "@destack/schema";
import { Scope, Subject } from "@destack/sync";
import { RelayError } from "../error/index.ts";
import relayPackage from "../package.ts";
import type { TunnelAttachment } from "./tunnel.ts";

/** The universe's tunnel tokens, which a residency's machines open and renew their tunnels with. */
export class TunnelToken {
    /** Verify the universe's tokens for the relay against the replicated keys. */
    readonly #verifier: TokenVerifier;
    /** The residency whose accounts' machines tunnel to the relay. */
    readonly #residency: ResidencyCode;
    /** The replica of the universe's identities and residencies, whose keys sign the tokens. */
    readonly #replica: Pick<Snapshot, "read">;
    /** The WebCrypto reading the universe's keys. */
    readonly #crypto: Environment["crypto"];

    /** Verify the universe's tokens for a residency's relay by the replicated keys. */
    constructor(
        issuer: string,
        code: ResidencyCode,
        replica: Pick<Snapshot, "read">,
        environment: Pick<Environment, "clock" | "crypto" | "network" | "telemetry">,
    ) {
        // take the residency, the replica and the WebCrypto
        this.#residency = code;
        this.#replica = replica;
        this.#crypto = environment.crypto;

        // verify the universe's tokens alone, by its current keys
        this.#verifier = new TokenVerifier({
            issuers: async (name) =>
                name === issuer ? await Identity.trust(replica, name, issuer) : undefined,
            environment,
        });
    }

    /** Verify a machine's token, answering the machine, its token's lapse and its signing key. */
    async verify(token: string, now: number): Promise<TunnelAttachment> {
        // require a machine's token
        const { caller, credential } = await this.#verifier.verify(token, {
            audience: relayPackage.id,
            now,
        });
        const { subject } = caller;
        if (!principal.machine.is(subject)) {
            throw RelayError.of(
                "MACHINE_DENIED",
                "tunnel is forbidden: only machines can open tunnels",
            );
        }

        // require the machine inside an account of the relay's residency, as the universe vouches
        const resident = await this.#replica
            .read(residency.table)
            .findFirst({ where: { code: this.#residency } });
        const contained = present(resident, `the residency '${this.#residency}'`);
        const set = {
            ...residency.reference(Scope.universe.id, contained.id),
            relation: CONTAINED,
        };
        if (!caller.groups.some((group) => Subject.key(group) === Subject.key(set))) {
            throw RelayError.of(
                "RESIDENCY_DENIED",
                `machine '${subject.id}' is forbidden: it tunnels through the relay of its account's residency`,
            );
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
