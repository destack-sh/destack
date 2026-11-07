import { InstallationOrigin, SpaceAddress } from "@destack/account/address";
import { argument, t } from "@destack/locale";
import { present, schema, type Identifier } from "@destack/schema";
import { Badge } from "@destack/ui/badge";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@destack/ui/empty";
import { Item, ItemContent, ItemDescription, ItemGroup, ItemTitle } from "@destack/ui/item";
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
import {
    INSTALLATION_PARAMETER,
    OPEN_PATH,
    SPACE_PARAMETER,
    VIEW_PARAMETER,
} from "@destack/view/declare";
import type { ViewEntry } from "@destack/view/palette";
import { space } from "@destack/account/object";
import { SpaceCatalog } from "../command/catalog.ts";

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
    const [views, setViews] = createSignal<readonly ViewEntry[]>([]);
    const [failure, setFailure] = createSignal<string | undefined>(undefined);
    createEffect(
        () => properties.id,
        (id) => {
            SpaceCatalog.read({ id, address: properties.address }, origin, environment).then(
                (catalog) => {
                    setFailure(undefined);
                    setViews(catalog.entries({}, locale).filter((entry) => entry.kind === "view"));
                },
                (error: unknown) => {
                    globalThis.reportError(error);
                    setFailure(error instanceof Error ? error.message : String(error));
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
                                <a href={openPath(properties.address, view)}>{view.title}</a>
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
            </ItemContent>
        </Item>
    );
}

/** Address a view of an installation in a space through the page's open path. */
function openPath(address: string, view: ViewEntry): string {
    const query = new URLSearchParams({
        [SPACE_PARAMETER]: address,
        [INSTALLATION_PARAMETER]: view.installation,
        [VIEW_PARAMETER]: view.name,
    });

    return `${OPEN_PATH}?${query}`;
}
