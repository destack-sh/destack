import { Uuid } from "@destack/schema";
import { type Environment } from "@destack/environment";
import type { SimulatedClock } from "@destack/environment/simulation";
import type { Identifier } from "@destack/schema";
import { STRANGER_EMAIL, TestAccountStore, TestDns, type TestPerson } from "../src/test/index.ts";

/** Two shared accounts with an owner each and a stranger, claiming domains over a test DNS. */
export class TestDomain implements AsyncDisposable {
    /** The account package, with its people. */
    readonly accounts: TestAccountStore;
    /** The DNS domain claims verify through. */
    readonly dns: TestDns;
    /** The owner of the first account. */
    readonly owner: TestPerson;
    /** The owner of the second account. */
    readonly rival: TestPerson;
    /** A person of neither account. */
    readonly stranger: TestPerson;
    /** The first account. */
    readonly accountId: Identifier<"account">;
    /** The second account. */
    readonly otherId: Identifier<"account">;
    /** A domain of the scenario. */
    readonly name: string;

    /** Keep the scenario's people and accounts. */
    private constructor(
        accounts: TestAccountStore,
        dns: TestDns,
        people: Pick<TestDomain, "owner" | "rival" | "stranger" | "accountId" | "otherId">,
    ) {
        this.accounts = accounts;
        this.dns = dns;
        this.name = `acme-${Uuid.v4(accounts.environment.random).slice(0, 8)}.example.com`;
        this.owner = people.owner;
        this.rival = people.rival;
        this.stranger = people.stranger;
        this.accountId = people.accountId;
        this.otherId = people.otherId;
    }

    /** The clock the scenario runs at, standing still unless moved. */
    get clock(): SimulatedClock {
        return this.accounts.clock;
    }

    /** The world the scenario runs in, its accounts'. */
    get environment(): Environment {
        return this.accounts.environment;
    }

    /** Sign the people up, each owner creating a shared account. */
    static async open(): Promise<TestDomain> {
        // serve the account package over the test DNS at a clock standing still
        const dns = new TestDns();
        const accounts = await TestAccountStore.open({ resolver: dns });

        // sign the people up, and let each owner create an account
        const owner = await accounts.signIn("owner@example.com");
        const rival = await accounts.signIn("rival@example.com");
        const stranger = await accounts.signIn(STRANGER_EMAIL);

        return new TestDomain(accounts, dns, {
            owner,
            rival,
            stranger,
            accountId: (await accounts.createAccount(owner)).id,
            otherId: (await accounts.createAccount(rival)).id,
        });
    }

    /** Stop serving the account package. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.accounts[Symbol.asyncDispose]();
    }
}
