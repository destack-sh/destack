import { InstallationOrigin, SpaceAddress, OPEN_PATH } from "@destack/account/address";
import { Failure } from "@destack/error";
import { argument, t } from "@destack/locale";
import { present, schema, type Identifier } from "@destack/schema";
import { Badge } from "@destack/ui/badge";
import { Button } from "@destack/ui/button";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@destack/ui/empty";
import {
    Item,
    ItemActions,
    ItemContent,
    ItemDescription,
    ItemGroup,
    ItemTitle,
} from "@destack/ui/item";
import {
    createEffect,
    createSignal,
    For,
    Show,
    useAccount,
    useEnvironment,
    useLocale,
    useOrigin,
    useQuery,
    useView,
} from "@destack/view";
import { INSTALLATION_PARAMETER, SPACE_PARAMETER, VIEW_PARAMETER } from "@destack/view/declare";
import type { ViewEntry } from "@destack/view/palette";
import { space } from "@destack/account/object";
import { PackageMount } from "@destack/object";
import { createClient } from "@destack/object/client";
import objectPackage from "@destack/object/package";
import { RequestId } from "@destack/sync";
import { HOME_PATH } from "@destack/view/declare";
import { SpaceCatalog } from "../palette/catalog.ts";
import {
    allowAlways,
    readWaitingChanges,
    readWaitingPlans,
    type WaitingChange,
    type WaitingPlan,
} from "./approval.ts";

/** List the account's spaces by name, each with the views its apps mount. */
export function Spaces() {
    // read the handle of the account whose spaces the page's own origin serves
    const environment = useEnvironment();
    const page = new URL(environment.navigation.currentEntry.url);
    const domain = page.hostname.slice(page.hostname.indexOf(".") + 1);
    const own = present(InstallationOrigin.parse(page.hostname, domain), "the page's origin");
    const { handle } = SpaceAddress.origin(SpaceAddress.of(own));

    // follow the account's spaces outside the trash, by name
    const connected = schema.identifier("space").parse(useView().space);
    const account = useAccount({ space });
    const spaces = useQuery(
        account.query.space.findMany({
            where: { deletedAt: { isNull: true } },
            orderBy: { name: "asc" },
        }),
    );

    return (
        <Show when={spaces().length > 0} fallback={<NoSpaces />}>
            <ItemGroup>
                <For each={spaces()}>
                    {(entry) => (
                        <SpaceItem
                            id={entry.id}
                            address={SpaceAddress.of({ space: entry.name, handle })}
                            title={entry.title ?? entry.name}
                            isConnected={entry.id === connected}
                        />
                    )}
                </For>
            </ItemGroup>
        </Show>
    );
}

/** Explain how spaces appear before the first one does. */
function NoSpaces() {
    const locale = useLocale();

    return (
        <Empty>
            <EmptyHeader>
                <EmptyTitle>{locale.render(t`No spaces`)}</EmptyTitle>
                <EmptyDescription>
                    {locale.render(t`Spaces you create in this account appear here.`)}
                </EmptyDescription>
            </EmptyHeader>
        </Empty>
    );
}

/** Show one space by its title and address with its apps' views, marking the page's own. */
function SpaceItem(properties: {
    /** The space. */
    id: Identifier<"space">;
    /** The space's address. */
    address: string;
    /** The display text. */
    title: string;
    /** Whether the page runs in this space. */
    isConnected: boolean;
}) {
    // read the views the space's installations mount, from their builds' index
    const locale = useLocale();
    const origin = useOrigin();
    const environment = useEnvironment();
    const [views, setViews] = createSignal<readonly Pick<ViewEntry, "installation" | "name">[]>([]);
    const [failure, setFailure] = createSignal<string | undefined>(undefined);

    // read the plans the space's installations wait on, again after each approval
    const [waitingPlans, setWaitingPlans] = createSignal<readonly WaitingPlan[]>([]);
    const [approvalFailure, setApprovalFailure] = createSignal<string | undefined>(undefined);
    const [waitingChanges, setWaitingChanges] = createSignal<readonly WaitingChange[]>([]);
    const person = useView().user;
    const readApproval = (id: Identifier<"space">) =>
        Promise.all([
            readWaitingPlans(id, origin, environment),
            readWaitingChanges(id, person, origin, environment),
        ]).then(
            ([plans, changes]) => {
                setWaitingPlans(plans);
                setWaitingChanges(changes);
            },
            (error: unknown) => {
                globalThis.reportError(error);
                setApprovalFailure(Failure.of(error).message);
            },
        );
    const settled = (deciding: Promise<unknown>) =>
        deciding.then(
            () => {
                setApprovalFailure(undefined);
                void readApproval(properties.id);
            },
            (error: unknown) => setApprovalFailure(Failure.of(error).message),
        );
    const decide = (
        waiting: Pick<WaitingPlan, "space" | "approval">,
        decision: "approve" | "reject",
    ) => {
        const objects = createClient<typeof objectPackage.router>(
            objectPackage,
            origin(`${HOME_PATH}/${waiting.space}${PackageMount.path(objectPackage.id)}`),
            environment,
        );
        const request = { scope: waiting.space, id: waiting.approval };
        void settled(
            objects.approval[decision]({ ...request, requestId: RequestId.create(environment) }),
        );
    };
    const always = (
        waiting: WaitingChange & { readonly grant: NonNullable<WaitingChange["grant"]> },
    ) => void settled(allowAlways(waiting, person, origin, environment));
    createEffect(
        () => properties.id,
        (id) => void readApproval(id),
    );
    createEffect(
        () => properties.id,
        (id) => {
            SpaceCatalog.read({ id, address: properties.address }, origin, environment).then(
                (catalog) => {
                    setFailure(undefined);
                    setViews(
                        catalog.installations.flatMap((installed) =>
                            installed.main === undefined
                                ? []
                                : [{ installation: installed.name, name: installed.main }],
                        ),
                    );
                },
                (error: unknown) => {
                    globalThis.reportError(error);
                    setFailure(Failure.of(error).message);
                },
            );
        },
    );

    return (
        <Item variant="outline">
            <ItemContent>
                <ItemTitle>
                    {properties.title}
                    <Show when={properties.isConnected}>
                        <Badge>{locale.render(t`Connected`)}</Badge>
                    </Show>
                </ItemTitle>
                <ItemDescription>{properties.address}</ItemDescription>

                {/* Views, or why they cannot be read */}
                <Show
                    when={failure()}
                    fallback={
                        <For each={views()}>
                            {(view) => (
                                <a href={openPath(properties.address, view)}>{view.installation}</a>
                            )}
                        </For>
                    }
                >
                    {(message) => (
                        <ItemDescription>
                            {locale.render(
                                t`Cannot read this space's views: ${argument("message")}`,
                                {
                                    message: message(),
                                },
                            )}
                        </ItemDescription>
                    )}
                </Show>

                {/* Plans waiting for approval, each with its steps */}
                <For each={waitingPlans()}>
                    {(waiting) => (
                        <Item variant="muted">
                            <ItemContent>
                                <ItemTitle>
                                    {locale.render(t`${argument("name")} waits for approval`, {
                                        name: waiting.name,
                                    })}
                                </ItemTitle>
                                <For each={waiting.plan.steps}>
                                    {(step) => (
                                        <ItemDescription>
                                            {step.detail} <Badge>{step.risk}</Badge>
                                        </ItemDescription>
                                    )}
                                </For>
                            </ItemContent>
                            <ItemActions>
                                <Button onClick={() => decide(waiting, "approve")}>
                                    {locale.render(t`Approve`)}
                                </Button>
                                <Button variant="outline" onClick={() => decide(waiting, "reject")}>
                                    {locale.render(t`Reject`)}
                                </Button>
                            </ItemActions>
                        </Item>
                    )}
                </For>

                {/* Changes waiting for the person: once, from now on, or not */}
                <For each={waitingChanges()}>
                    {(waiting) => (
                        <Item variant="muted">
                            <ItemContent>
                                <ItemTitle>
                                    {locale.render(
                                        t`${argument("actor")} asks to ${argument("method")} ${argument("type")}`,
                                        {
                                            actor: waiting.actor.id,
                                            method: waiting.call.method,
                                            type: waiting.call.type,
                                        },
                                    )}
                                </ItemTitle>
                            </ItemContent>
                            <ItemActions>
                                <Button onClick={() => decide(waiting, "approve")}>
                                    {locale.render(t`Only this time`)}
                                </Button>
                                <Show when={waiting.grant}>
                                    {(grant) => (
                                        <Button
                                            variant="outline"
                                            onClick={() => always({ ...waiting, grant: grant() })}
                                        >
                                            {locale.render(t`Always allow`)}
                                        </Button>
                                    )}
                                </Show>
                                <Button variant="ghost" onClick={() => decide(waiting, "reject")}>
                                    {locale.render(t`Don't allow`)}
                                </Button>
                            </ItemActions>
                        </Item>
                    )}
                </For>

                {/* Why the plans and changes could not be read or decided */}
                <Show when={approvalFailure()}>
                    {(message) => <ItemDescription>{message()}</ItemDescription>}
                </Show>
            </ItemContent>
        </Item>
    );
}

/** Address a view of an installation in a space through the page's open path. */
function openPath(address: string, view: Pick<ViewEntry, "installation" | "name">): string {
    const query = new URLSearchParams({
        [SPACE_PARAMETER]: address,
        [INSTALLATION_PARAMETER]: view.installation,
        [VIEW_PARAMETER]: view.name,
    });

    return `${OPEN_PATH}?${query}`;
}
