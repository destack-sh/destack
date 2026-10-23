import { DomainChallenge } from "../object/domain.ts";
import type { DnsResolver } from "../server/dns-resolver.ts";

/** DNS as a test publishes its TXT records, recording each name it is asked for. */
export class TestDns implements DnsResolver {
    /** The TXT records it answers, by name. */
    readonly records = new Map<string, string[]>();
    /** The names it was asked for, in order. */
    readonly lookups: string[] = [];

    /** Read the TXT records of a name, recording the lookup. */
    async txt(name: string): Promise<string[]> {
        this.lookups.push(name);

        return this.records.get(name) ?? [];
    }

    /** Publish the challenge record of a domain claim beside the records its name has. */
    publish(claimed: { readonly scope: string; readonly name: string }): void {
        const challenge = DomainChallenge.of(claimed);
        const published = this.records.get(challenge.name) ?? [];
        this.records.set(challenge.name, [...published, challenge.value]);
    }
}
