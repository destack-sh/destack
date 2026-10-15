import type { CallOf, NextOf, ResultOf } from "@destack/object";
import { schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import type { DnsResolver } from "./dns-resolver.ts";
import { account } from "../object/account.ts";
import { Allowance } from "../object/allowance.ts";
import { Domain, DomainChallenge, domain } from "../object/domain.ts";
import { hostname } from "../object/hostname.ts";

/** What domain claims verify with: the resolver reading their challenge records. */
export interface DomainOptions {
    /** The resolver reading the challenge records. */
    readonly resolver: DnsResolver;
}

/** Implement domains. */
export function implementDomain(options: DomainOptions): typeof domain {
    return domain.implement({
        methods: {
            create: async (call, next) => {
                // refuse a payer whose plan allows no domains
                await Allowance.requireCustomDomains(call);

                return next();
            },
            verify: {
                prepare: (call) => verify(call, options),
                handle: (call) => call.update({ verifiedAt: call.now }),
            },
            delete: remove,
        },
    });
}

/** Require an unverified claim, its domain free of other verified claims. */
async function verify(
    call: Pick<CallOf<typeof domain, "verify">, "target" | "read">,
    options: DomainOptions,
): Promise<DomainChallenge> {
    // read the claim and any verified claim of its domain
    const claimed = call.target;
    const verified = await call
        .read(domain)
        .findFirst({ where: { name: claimed.name, verifiedAt: { isNotNull: true } } });

    // refuse a claim verified before
    if (claimed.verifiedAt !== null) {
        throw new CallError("CONFLICT", { message: `${claimed.name} is verified` });
    }
    // refuse a domain another owner verified
    else if (verified !== undefined) {
        throw new CallError("CONFLICT", {
            message: `another owner verified ${claimed.name}`,
        });
    }

    // require the challenge record
    const challenge = DomainChallenge.of(claimed);
    const records = await options.resolver.txt(challenge.name);
    if (!records.includes(challenge.value)) {
        throw new CallError("PRECONDITION_FAILED", {
            message: `publish a TXT record ${challenge.name} with ${challenge.value} first`,
        });
    }

    return challenge;
}

/** Delete a domain, refusing while a hostname below it serves an account it covers. */
async function remove(
    call: CallOf<typeof domain, "delete">,
    next: NextOf<typeof domain, "delete">,
): Promise<ResultOf<typeof domain, "delete">> {
    // read the hostnames of the domain's account or its organisation's accounts
    const { name, scope } = call.target;
    const owner = schema.identifier("account").safeParse(scope);
    const accounts = owner.success
        ? [owner.data]
        : (await call.read(account).findMany({ where: { scope } })).map((row) => row.id);
    const rows = await call.read(hostname).findMany({ where: { scope: { in: accounts } } });

    // refuse while any lie below the domain, naming them all
    const covered = rows
        .filter((row) => Domain.covers(name, row.name))
        .map((row) => row.name)
        .toSorted();
    if (covered.length > 0) {
        throw new CallError("PRECONDITION_FAILED", {
            message: `hostnames serve below ${name}: ${covered.join(", ")}`,
        });
    }

    return next();
}
