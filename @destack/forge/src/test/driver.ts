import type { Clock } from "@destack/environment";
import type { SimulatedTelemetry } from "@destack/environment/simulation";
import { Worktree } from "@destack/git/test";
import type { Given, SetStep } from "@destack/package/declare";
import { type Identifier, type JsonValue, present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import type { Driver } from "@destack/test";
import {
    forgeInteraction,
    type ForgeObservation,
    type ForgeStep,
} from "../scenario/interaction.ts";
import { ACCOUNT_ID, TestForge } from "./forge.ts";

/** The branch the driver commits, pushes and pulls. */
const BRANCH = "main";

/** A driver that plays forge scenarios as the publishing account's owner, with git on the host. */
export class ForgeDriver implements Driver<ForgeStep, ForgeObservation> {
    /** The interaction the driver plays. */
    static readonly interaction = forgeInteraction;
    /** The settings its worlds run with: SQLite in WAL, synchronous NORMAL, as in production. */
    static readonly configuration = { durability: "normal" };

    /** The forge's world. */
    readonly #forge: TestForge;
    /** The working tree the owner commits in. */
    readonly #worktree: Worktree;
    /** The local commits, oldest first. */
    readonly #commits: string[] = [];
    /** The clones of the repository, oldest first. */
    readonly #clones: Worktree[] = [];
    /** The repository the steps work on, absent before the first create. */
    #repository: Identifier<"repository"> | undefined;
    /** The repositories created so far. */
    #created = 0;
    /** The local commits pushed so far. */
    #pushed = 0;
    /** The clones pulled into so far. */
    #pulled = 0;
    /** The checkout last cloned or pulled. */
    #checkout: Worktree | undefined;

    /** Take a forge's world and the owner's working tree. */
    constructor(forge: TestForge, worktree: Worktree) {
        this.#forge = forge;
        this.#worktree = worktree;
    }

    /** The world's clock. */
    get clock(): Clock {
        return this.#forge.environment.clock;
    }

    /** The telemetry the world records. */
    get telemetry(): SimulatedTelemetry {
        return this.#forge.simulation.telemetry;
    }

    /** Open a forge's world and an empty working tree, given no examples. */
    static async start(_given: Given): Promise<ForgeDriver> {
        return new ForgeDriver(await TestForge.open(), await Worktree.open());
    }

    /** Take one step as the owner. */
    async act(step: SetStep | ForgeStep): Promise<void> {
        const owner = this.#forge.client(this.#forge.owner);
        const spaceId = this.#forge.space;

        // refuse the set step, as a forge world has no example properties
        if (step.action === "set") {
            throw new TypeError("a forge world takes no example properties");
        }
        // create a platform repository and work on it from here on
        else if (step.action === "create") {
            this.#created += 1;
            const created = await owner.repository.create({
                spaceId,
                accountId: ACCOUNT_ID,
                requestId: RequestId.create(this.#forge.environment),
                name: `site-${this.#created}`,
                hosting: "platform",
            });
            this.#repository = created.id;
        }
        // commit locally
        else if (step.action === "commit") {
            for (let index = 0; index < step.count; index += 1) {
                const file = `page-${this.#commits.length + 1}.md`;
                this.#commits.push(await this.#worktree.commit(file, `# ${file}\n`));
            }
        }
        // push the next local commit through a write lease
        else if (step.action === "push") {
            const commit = present(this.#commits[this.#pushed], "a local commit to push");
            const lease = await owner.repository.open({ spaceId, id: this.#id(), mode: "write" });
            await this.#worktree.push(lease.url, `${commit}:refs/heads/${BRANCH}`);
            this.#pushed += 1;
        }
        // clone through a read lease
        else if (step.action === "clone") {
            const lease = await owner.repository.open({ spaceId, id: this.#id(), mode: "read" });
            this.#checkout = await Worktree.clone(lease.url);
            this.#clones.push(this.#checkout);
        }
        // pull into the next clone through a read lease
        else {
            const clone = present(this.#clones[this.#pulled], "a clone to pull into");
            const lease = await owner.repository.open({ spaceId, id: this.#id(), mode: "read" });
            await clone.pull(lease.url, BRANCH);
            this.#pulled += 1;
            this.#checkout = clone;
        }
    }

    /** Read whether the checkout last cloned or pulled is at the commit last pushed. */
    async observe(_observation: ForgeObservation): Promise<JsonValue> {
        const checkout = present(this.#checkout, "a cloned or pulled checkout");

        return (await checkout.head()) === this.#commits[this.#pushed - 1];
    }

    /** Read the fingerprint of everything the world did so far. */
    fingerprint(): number {
        return this.#forge.simulation.fingerprint;
    }

    /** Close the forge's home. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#forge.home[Symbol.asyncDispose]();
    }

    /** Read the repository the steps work on. */
    #id(): Identifier<"repository"> {
        return present(this.#repository, "a created repository");
    }
}
