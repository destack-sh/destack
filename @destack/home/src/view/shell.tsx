import { InstallationOrigin, SpaceAddress } from "@destack/account/address";
import * as accountObject from "@destack/account/object";
import { Icon } from "@destack/ui/icon";
import { t } from "@destack/locale";
import { installation } from "@destack/object";
import { rollout } from "@destack/stack/object";
import { type Identifier, present } from "@destack/schema";
import * as style from "@destack/style";
import { color, size, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuTrigger,
} from "@destack/ui/dropdown-menu";
import {
    Sidebar,
    SidebarContent,
    SidebarFooter,
    SidebarGroup,
    SidebarHeader,
    SidebarInset,
    SidebarMenu,
    SidebarMenuButton,
    SidebarMenuItem,
    SidebarProvider,
    SidebarRail,
    SidebarSeparator,
    SidebarTrigger,
} from "@destack/ui/sidebar";
import {
    createEffect,
    createSignal,
    For,
    Show,
    useAccount,
    useEnvironment,
    useLocale,
    useConnect,
    useQuery,
    useScope,
} from "@destack/view";
import { Frame } from "@destack/view/frame";
import { AppMark } from "./app.tsx";
import { parseAsString, useQueryState } from "@destack/view/router";
import { Failure } from "@destack/error";
import { SpaceCatalog } from "../palette/catalog.ts";
import { Spaces } from "./space.tsx";

/** The space Home opens before the person picks another: their home. */
const HOME_SPACE_NAME = "home";

/** The shell's layout: the rail of apps beside the open app, which fills the rest. */
const styles = style.create({
    bar: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: 1,
        display: "flex",
        gap: space[2],
        minBlockSize: "2.75rem",
        paddingInline: space[3],
    },
    main: {
        display: "grid",
        flex: 1,
        minBlockSize: 0,
    },
    overview: {
        marginInline: "auto",
        maxInlineSize: "56rem",
        padding: space[6],
        inlineSize: "100%",
    },
    status: {
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        height: size[3],
        paddingInline: space[3],
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        backgroundColor: color.muted,
        color: color.mutedForeground,
        whiteSpace: "nowrap",
        boxSizing: "border-box",
    },
    item: {
        display: "inline-flex",
        alignItems: "center",
        gap: space[1],
    },
});

/** Show the person's spaces as a rail of their apps, and the app they open in a frame. */
export function Shell() {
    // read the handle of the account whose spaces the page's own origin serves
    const environment = useEnvironment();
    const locale = useLocale();
    const page = new URL(environment.navigation.currentEntry.url);
    const domain = page.hostname.slice(page.hostname.indexOf(".") + 1);
    const own = present(InstallationOrigin.parse(page.hostname, domain), "the page's origin");
    const { handle } = SpaceAddress.origin(SpaceAddress.of(own));

    // follow the account's spaces, and the space and app the address names
    const account = useAccount({ space: accountObject.space });
    const spaces = useQuery(
        account.query.space.findMany({
            where: { deletedAt: { isNull: true } },
            orderBy: { name: "asc" },
        }),
    );
    const [spaceName, setSpaceName] = useQueryState("space", parseAsString);
    const [app, setApp] = useQueryState("app", parseAsString);
    const opened = () => spaces().find((each) => each.name === (spaceName() ?? HOME_SPACE_NAME));
    const address = (current: { readonly name: string }) =>
        SpaceAddress.of({ space: current.name, handle });

    return (
        <SidebarProvider defaultOpen={false}>
            <Sidebar collapsible="icon">
                {/* Space switcher */}
                <SidebarHeader>
                    <SidebarMenu>
                        <SidebarMenuItem>
                            <DropdownMenu>
                                <DropdownMenuTrigger variant="ghost">
                                    <Icon name="cube" />
                                    {opened()?.title ?? opened()?.name ?? HOME_SPACE_NAME}
                                </DropdownMenuTrigger>
                                <DropdownMenuContent>
                                    <DropdownMenuLabel>
                                        {locale.render(t`Spaces`)}
                                    </DropdownMenuLabel>
                                    <For each={spaces()}>
                                        {(each) => (
                                            <DropdownMenuItem
                                                onSelect={() => {
                                                    void setSpaceName(each.name);
                                                    void setApp(null);
                                                }}
                                            >
                                                {each.title ?? each.name}
                                            </DropdownMenuItem>
                                        )}
                                    </For>
                                </DropdownMenuContent>
                            </DropdownMenu>
                        </SidebarMenuItem>
                    </SidebarMenu>
                </SidebarHeader>
                <SidebarSeparator />

                {/* The space's apps */}
                <SidebarContent>
                    <Show when={opened()}>
                        {(current) => (
                            <AppMenu
                                id={current().id}
                                address={address(current())}
                                open={app()}
                                onOpen={(name) => void setApp(name)}
                            />
                        )}
                    </Show>
                </SidebarContent>

                {/* Overview and settings */}
                <SidebarFooter>
                    <SidebarMenu>
                        <SidebarMenuItem>
                            <SidebarMenuButton
                                active={app() === null}
                                onClick={() => void setApp(null)}
                            >
                                <Icon name="squares-four" />
                                {locale.render(t`Overview`)}
                            </SidebarMenuButton>
                        </SidebarMenuItem>
                        <SidebarMenuItem>
                            <SidebarMenuButton href="/settings">
                                <Icon name="gear-six" />
                                {locale.render(t`Settings`)}
                            </SidebarMenuButton>
                        </SidebarMenuItem>
                    </SidebarMenu>
                </SidebarFooter>
                <SidebarRail />
            </Sidebar>

            {/* The open app, else the overview of the account's spaces */}
            <SidebarInset>
                <header {...style.attrs(styles.bar)}>
                    <SidebarTrigger />
                    <span {...style.attrs(text.sm)}>{app() ?? locale.render(t`Overview`)}</span>
                </header>
                <div {...style.attrs(styles.main)}>
                    <Show
                        when={opened() !== undefined && app() !== null ? app() : undefined}
                        fallback={
                            <div {...style.attrs(styles.overview)}>
                                <Spaces />
                            </div>
                        }
                    >
                        {(name) => (
                            <AppFrame
                                id={present(opened(), "the opened space").id}
                                address={address(present(opened(), "the opened space"))}
                                installation={name()}
                            />
                        )}
                    </Show>
                </div>

                {/* The open space at the start, its account at the end */}
                <footer {...style.attrs(text.xs, styles.status)}>
                    <span {...style.attrs(styles.item)}>
                        <Icon name="cube" />
                        {opened() === undefined
                            ? handle
                            : address(present(opened(), "the opened space"))}
                    </span>
                    <span {...style.attrs(styles.item)}>
                        <Icon name="user" />@{handle}
                    </span>
                </footer>
            </SidebarInset>
        </SidebarProvider>
    );
}

/** List a space's apps that open a view, live, each opening its view beside the rail. */
function AppMenu(properties: {
    /** The space. */
    id: Identifier<"space">;
    /** The space's address. */
    address: string;
    /** The installation open beside the rail, null for none. */
    open: string | null;
    /** Open an installation's view. */
    onOpen: (name: string) => void;
}) {
    // read the space's installations again at each change of them
    const locale = useLocale();
    const catalog = useCatalog(
        () => properties.id,
        () => properties.address,
    );

    return (
        <SidebarGroup>
            <SidebarMenu>
                <For each={catalog()?.installations.filter((each) => each.main !== undefined)}>
                    {(each) => (
                        <SidebarMenuItem>
                            <SidebarMenuButton
                                active={properties.open === each.name}
                                onClick={() => properties.onOpen(each.name)}
                            >
                                <AppMark
                                    title={SpaceCatalog.title(each, locale)}
                                    color={each.presentation?.color}
                                />
                                {SpaceCatalog.title(each, locale)}
                            </SidebarMenuButton>
                        </SidebarMenuItem>
                    )}
                </For>
            </SidebarMenu>
        </SidebarGroup>
    );
}

/** Frame an installation's main view, once the space's catalog names it. */
function AppFrame(properties: {
    /** The space. */
    id: Identifier<"space">;
    /** The space's address. */
    address: string;
    /** The installation to open. */
    installation: string;
}) {
    // find the installation's main view in the space's catalog
    const catalog = useCatalog(
        () => properties.id,
        () => properties.address,
    );
    const entry = () =>
        catalog()?.installations.find((each) => each.name === properties.installation);

    // follow the revision the installation serves, which reloads the frame as a preview's build moves
    const scoped = useScope(() => properties.id, { installation, rollout });
    const installed = useQuery(() =>
        scoped().query.installation.findMany({ where: { name: properties.installation } }),
    );
    const rollouts = useQuery(() => scoped().query.rollout.findMany({}));
    const revision = () => {
        const [found] = installed();
        const served = rollouts().find((each) => each.installationId === found?.id);

        return served?.servedRevisionId ?? undefined;
    };

    return (
        <Show when={entry()?.main}>
            {(view) => (
                <Frame
                    address={{
                        space: properties.address,
                        installation: properties.installation,
                        view: view(),
                        path: "/",
                    }}
                    title={properties.installation}
                    revision={revision()}
                />
            )}
        </Show>
    );
}

/** Read a space's catalog, again at each change of its installations. */
function useCatalog(id: () => Identifier<"space">, address: () => string) {
    // follow the space's installations, reading its catalog at each change
    const connect = useConnect();
    const [catalog, setCatalog] = createSignal<SpaceCatalog | undefined>(undefined);
    const scoped = useScope(id, { installation });
    const installations = useQuery(() => scoped().query.installation.findMany({}));
    createEffect(
        () => [id(), address(), installations()] as const,
        ([current, at]) => {
            SpaceCatalog.read({ id: current, address: at }, connect).then(
                setCatalog,
                (error: unknown) => globalThis.reportError(Failure.of(error)),
            );
        },
    );

    return catalog;
}
