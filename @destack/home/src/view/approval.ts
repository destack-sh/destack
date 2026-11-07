import type { Environment } from "@destack/environment";
import { Observation, PackageMount } from "@destack/object";
import { type ClientOptions, createClient } from "@destack/object/client";
import objectPackage from "@destack/object/package";
import type { Plan } from "@destack/resource";
import { type Identifier, present } from "@destack/schema";
import stackPackage from "@destack/stack/package";
import { HOME_PATH } from "@destack/view/declare";

/** A plan an installation of a space waits on, as a person reviews it. */
export interface WaitingPlan {
    /** The space. */
    readonly space: Identifier<"space">;
    /** The installation waiting. */
    readonly installation: Identifier<"installation">;
    /** The installation's name. */
    readonly name: string;
    /** The plan as its controller planned it. */
    readonly plan: Plan;
    /** The plan's digest, which approving it names. */
    readonly digest: string;
    /** The approval the plan's change request asks of the space's managers. */
    readonly approval: Identifier<"approval">;
}

/** Read the plans a space's installations wait on, each with the digest approving it names. */
export async function readWaitingPlans(
    space: Identifier<"space">,
    origin: (path: string) => ClientOptions,
    environment: Environment,
): Promise<WaitingPlan[]> {
    // list the space's installations, page by page
    const objects = createClient<typeof objectPackage.router>(
        objectPackage,
        origin(`${HOME_PATH}/${space}${PackageMount.path(objectPackage.id)}`),
        environment,
    );
    const items = [];
    let cursor: string | undefined;
    do {
        const page = await objects.installation.list({
            scope: space,
            ...(cursor === undefined ? {} : { cursor }),
        });
        items.push(...page.items);
        cursor = page.cursor ?? undefined;
    } while (cursor !== undefined);

    // read the plan of each named installation awaiting approval
    const stack = stackClient(space, origin, environment);
    const awaiting = items.filter(
        (entry) => entry.name !== null && Observation.progress(entry) === "approval",
    );
    const read = await Promise.all(
        awaiting.map(async (entry) => {
            // read the plan with the approval its change request asks, none before it opens
            const waiting = await stack.rollout.plan({ spaceId: space, id: entry.id });
            const { approval } = waiting;
            const name = present(entry.name, `the name of installation ${entry.id}`);

            return approval === null
                ? []
                : [{ space, installation: entry.id, name, ...waiting, approval }];
        }),
    );

    return read.flat();
}

/** Open the stack package's client in a space. */
function stackClient(
    space: Identifier<"space">,
    origin: (path: string) => ClientOptions,
    environment: Environment,
) {
    return createClient<typeof stackPackage.router>(
        stackPackage,
        origin(`${HOME_PATH}/${space}${PackageMount.path(stackPackage.id)}`),
        environment,
    );
}
