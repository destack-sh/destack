import type { Environment } from "@destack/environment";
import { GrantPermission } from "@destack/access";
import { changeRequest, ListPage, Observation, PackageMount } from "@destack/object";
import { type ClientOptions, createClient } from "@destack/object/client";
import objectPackage from "@destack/object/package";
import type { Plan } from "@destack/resource";
import { type Identifier, present } from "@destack/schema";
import stackPackage from "@destack/stack/package";
import { type Call, ObjectReference, RequestId, type Subject } from "@destack/sync";
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

/** A change a space's change request asks the person to approve, as they decide it. */
export interface WaitingChange {
    /** The space. */
    readonly space: Identifier<"space">;
    /** The approval asking the person. */
    readonly approval: Identifier<"approval">;
    /** Who acts for the requester, the requester itself without actors. */
    readonly actor: Subject;
    /** The call the change makes once approved. */
    readonly call: Call;
    /** The permission allowing the call from now on adds to the grant, null where none does. */
    readonly grant: GrantPermission | null;
}

/** Read the changes a space's change requests ask the person to approve, past the plans. */
export async function readWaitingChanges(
    space: Identifier<"space">,
    person: Subject,
    origin: (path: string) => ClientOptions,
    environment: Environment,
): Promise<WaitingChange[]> {
    // list the approvals asking the person on change requests
    const objects = createClient<typeof objectPackage.router>(
        objectPackage,
        origin(`${HOME_PATH}/${space}${PackageMount.path(objectPackage.id)}`),
        environment,
    );
    const asking = await Array.fromAsync(
        ListPage.all(objects.approval.list, {
            scope: space,
            where: {
                ...ObjectReference.columns("approver", person),
                parentType: changeRequest.name,
                state: "requested",
            },
        }),
    );

    // read each one's change request, keeping the open ones
    const changes = await Array.fromAsync(
        ListPage.all(objects.changeRequest.list, {
            scope: space,
            where: { id: { in: asking.map((each) => each.parentId) }, state: "open" },
        }),
    );

    return changes.flatMap((change) => {
        const asked = asking.find((each) => each.parentId === change.id);

        return asked === undefined
            ? []
            : [
                  {
                      space,
                      approval: asked.id,
                      actor: change.actors.at(-1) ?? ObjectReference.at("requester", change),
                      call: change.call,
                      grant: change.grant ?? null,
                  },
              ];
    });
}

/** Allow a waiting change's call from now on: grow the actor's grant by it, then approve. */
export async function allowAlways(
    waiting: WaitingChange & { readonly grant: GrantPermission },
    person: Subject,
    origin: (path: string) => ClientOptions,
    environment: Environment,
): Promise<void> {
    // read the person's grant to the actor, at the universe
    const universe = createClient<typeof objectPackage.router>(
        objectPackage,
        origin(`${HOME_PATH}/${person.id}${PackageMount.path(objectPackage.id)}`),
        environment,
    );
    const { items } = await universe.grant.list({
        scope: person.id,
        where: ObjectReference.columns("delegate", waiting.actor),
    });
    const [standing] = items;

    // grant it with the change's permission beside the standing ones, then approve the change
    await universe.grant.create({
        scope: person.id,
        requestId: RequestId.create(environment),
        delegate: waiting.actor,
        permissions: GrantPermission.merge(standing?.permissions ?? [], [waiting.grant]),
    });
    const space = createClient<typeof objectPackage.router>(
        objectPackage,
        origin(`${HOME_PATH}/${waiting.space}${PackageMount.path(objectPackage.id)}`),
        environment,
    );
    await space.approval.approve({
        scope: waiting.space,
        id: waiting.approval,
        requestId: RequestId.create(environment),
    });
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
