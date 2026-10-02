import { and, eq, isNotNull } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import type { Call } from "@destack/object";
import type { DnsResolver } from "../../dns/index.ts";
import * as base from "../../object/domain.ts";

/** A call on a domain. */
type DomainCall = Call<typeof base.domain.table>;

/** Verify domain claims through the TXT records their administrators publish. */
export class DomainVerifier {
    /** The resolver reading the challenge records. */
    readonly resolver: DnsResolver;

    /** Verify through a resolver. */
    constructor(resolver: DnsResolver) {
        this.resolver = resolver;
    }

    /** Give the domain object its verification. */
    handle(): typeof base.domain {
        return base.domain.handle({
            verify: {
                prepare: (call) => this.#prove(call),
                // TODO #Incomplete: recheck verified domains periodically and withdraw lapsed claims with the edge's route reconciler
                effect: (call) => call.update({ verifiedAt: call.now }),
            },
        });
    }

    /** Require the hostname free of other verified claims, and its challenge record. */
    async #prove(call: DomainCall): Promise<void> {
        // refuse a hostname another account verified
        const claimed = call.target!;
        const table = base.domain.table;
        const [verified] = await call.database
            .select({ id: table.id })
            .from(table)
            .where(and(eq(table.hostname, claimed.hostname), isNotNull(table.verifiedAt)))
            .limit(1);
        if (verified !== undefined && verified.id !== claimed.id) {
            throw new ServiceError("CONFLICT", {
                message: `another account verified ${claimed.hostname}`,
            });
        }

        // require the challenge record
        const challenge = base.DomainChallenge.of(claimed);
        const records = await this.resolver.txt(challenge.name);
        if (!records.includes(challenge.value)) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `publish a TXT record ${challenge.name} with ${challenge.value} first`,
            });
        }
    }
}
