import { CONTAINED, principal } from "@destack/access";
import { TokenVerifier } from "@destack/access/token";
import { Identity, region, type RegionCode } from "@destack/account/object";
import type { Snapshot } from "@destack/db/database";
import type { Environment } from "@destack/environment";
import { Jws } from "@destack/identity";
import { present, schema } from "@destack/schema";
import { Scope, Subject } from "@destack/sync";
import { RelayError } from "../error/index.ts";
import relayPackage from "../package.ts";
import type { TunnelAttachment } from "./tunnel.ts";

/** The universe's tunnel tokens, which a region's machines open and renew their tunnels with. */
export class TunnelToken {
    /** Verify the universe's tokens for the relay against the replicated keys. */
    readonly #verifier: TokenVerifier;
    /** The region whose accounts' machines tunnel to the relay. */
    readonly #region: RegionCode;
    /** The replica of the universe's identities and regions, whose keys sign the tokens. */
    readonly #replica: Pick<Snapshot, "read">;
    /** The WebCrypto reading the universe's keys. */
    readonly #crypto: Environment["crypto"];

    /** Verify the universe's tokens for a region's relay by the replicated keys. */
    constructor(
        issuer: string,
        code: RegionCode,
        replica: Pick<Snapshot, "read">,
        environment: Pick<Environment, "clock" | "crypto" | "network" | "telemetry">,
    ) {
        // take the region, the replica and the WebCrypto
        this.#region = code;
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

        // require the machine inside an account of the relay's region, as the universe vouches
        const served = await this.#replica
            .read(region.table)
            .findFirst({ where: { code: this.#region } });
        const contained = present(served, `the region '${this.#region}'`);
        const set = {
            ...region.reference(Scope.universe.id, contained.id),
            relation: CONTAINED,
        };
        if (!caller.groups.some((group) => Subject.key(group) === Subject.key(set))) {
            throw RelayError.of(
                "REGION_DENIED",
                `machine '${subject.id}' is forbidden: it tunnels through the relay of its account's region`,
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
