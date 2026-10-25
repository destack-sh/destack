import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { schema } from "@destack/schema";
import type { DnsResolver } from "./dns-resolver.ts";
import { account } from "../object/account.ts";
import { Entitlement } from "../object/entitlement.ts";
import { Domain, DomainChallenge, domain } from "../object/domain.ts";
import { hostname } from "../object/hostname.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

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
                await Entitlement.requireCustomDomains(call);

                return next();
            },
            verify: {
                prepare: (context) => verify(context, options),
                handle: (context) => context.update({ verifiedAt: context.now }),
            },
            delete: remove,
        },
    });
}

/** Require an unverified claim, its domain free of other verified claims. */
async function verify(
    context: Pick<MethodContextOf<typeof domain, "verify">, "target" | "read">,
    options: DomainOptions,
): Promise<DomainChallenge> {
    // read the claim and any verified claim of its domain
    const claimed = context.target;
    const verified = await context
        .read(domain)
        .findFirst({ where: { name: claimed.name, verifiedAt: { isNotNull: true } } });

    // refuse a claim verified before
    if (claimed.verifiedAt !== null) {
        throw AccountError.of("DOMAIN_VERIFIED", `${claimed.name} is already verified`);
    }
    // refuse a domain another owner verified
    else if (verified !== undefined) {
        throw AccountError.of("DOMAIN_TAKEN", `another owner verified ${claimed.name}`);
    }

    // require the challenge record
    const challenge = DomainChallenge.of(claimed);
    const records = await options.resolver.txt(challenge.name);
    if (!records.includes(challenge.value)) {
        throw AccountError.of(
            "CHALLENGE_UNPUBLISHED",
            `publish a TXT record ${challenge.name} with ${challenge.value} first`,
        );
    }

    return challenge;
}

/** Delete a domain, refusing while a hostname below it serves an account it covers. */
async function remove(
    context: MethodContextOf<typeof domain, "delete">,
    next: NextOf<typeof domain, "delete">,
): Promise<ResultOf<typeof domain, "delete">> {
    // read the hostnames of the domain's account or its organization's accounts
    const { name, scope } = context.target;
    const owner = schema.identifier("account").safeParse(scope);
    const accounts = owner.success
        ? [owner.data]
        : (await context.read(account).findMany({ where: { scope } })).map((row) => row.id);
    const rows = await context.read(hostname).findMany({ where: { scope: { in: accounts } } });

    // refuse while any lie below the domain, listing them all
    const covered = rows
        .filter((row) => Domain.covers(name, row.name))
        .map((row) => row.name)
        .toSorted();
    if (covered.length > 0) {
        throw AccountError.of(
            "HOSTNAMES_IN_USE",
            `hostnames below ${name} are in use: ${covered.join(", ")}`,
        );
    }

    return next();
}
