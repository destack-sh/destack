import type { Moment } from "../_generated/runtime/world/lineage/moment.js";
import type * as worldPolicy from "../_generated/runtime/world/policy/policy.js";
import type {
    Rule,
    RuleId,
} from "../_generated/runtime/world/policy/rule.js";

import type { World } from "./world.js";

/** One World-bound runtime Policy. */
export class Policy {
    /** Owning World. */
    readonly world: World;

    /** Bind one Policy to its World. */
    constructor(world: World) {
        this.world = world;
    }

    /** Read the Policy at one Moment. */
    async read(moment?: Moment): Promise<worldPolicy.Policy> {
        const response = await this.world.client.readPolicy({
            worldId: this.world.id,
            moment,
        });

        return response.value;
    }

    /** Replace the active Policy. */
    async replace(replacement: worldPolicy.Policy): Promise<void> {
        await this.world.client.replacePolicy({
            worldId: this.world.id,
            policy: replacement,
        });
    }

    /** Add one Rule to the active Policy. */
    async add(rule: Rule): Promise<void> {
        await this.world.client.addRule({ worldId: this.world.id, rule });
    }

    /** Replace one Rule in the active Policy. */
    async replaceRule(rule: Rule): Promise<void> {
        await this.world.client.replaceRule({ worldId: this.world.id, rule });
    }

    /** Remove one Rule from the active Policy. */
    async remove(ruleId: RuleId): Promise<void> {
        await this.world.client.removeRule({
            worldId: this.world.id,
            ruleId,
        });
    }
}

export type { Rule, RuleId };
