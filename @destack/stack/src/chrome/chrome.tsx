import { InstallationOrigin } from "@destack/account/address";
import * as accounts from "@destack/account/object";
import type { IconName } from "@destack/icon";
import { t } from "@destack/locale";
import { PackageMount } from "@destack/object";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, space, swatch, weight } from "@destack/theme/tokens.stylex";
import { Avatar, AvatarFallback } from "@destack/ui/avatar";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuTrigger,
} from "@destack/ui/dropdown-menu";
import { Icon, LazyIcon } from "@destack/ui/icon";
import { KbdShortcut } from "@destack/ui/kbd";
import {
    Sidebar,
    SidebarContent,
    SidebarFooter,
    SidebarGroup,
    SidebarGroupLabel,
    SidebarHeader,
    SidebarInset,
    SidebarMenu,
    SidebarMenuBadge,
    SidebarMenuButton,
    SidebarMenuItem,
    SidebarMenuSub,
    SidebarMenuSubButton,
    SidebarMenuSubItem,
    SidebarProvider,
    SidebarRail,
    SidebarTrigger,
} from "@destack/ui/sidebar";
import {
    createEffect,
    createMemo,
    createSignal,
    For,
    type JSX,
    palette,
    Show,
    useActions,
    useConnect,
    useEnvironment,
    useLocale,
    useQuery,
} from "@destack/view";
import { ScopePath } from "@destack/view/declare";
import { installation } from "../object/index.ts";
import stackPackage from "../package.ts";
import { SpaceApp } from "./app.ts";

/** The styles of the chrome: the space's mark, the header bar and its title. */
const styles = style.create({
    mark: (fill: string) => ({
        display: "grid",
        placeItems: "center",
        flexShrink: 0,
        width: space[5],
        height: space[5],
        borderRadius: radius[2],
        backgroundColor: fill,
        color: color.background,
        fontWeight: weight.semibold,
    }),
    space: {
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
        fontWeight: weight.medium,
    },
    account: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        paddingInline: space[2],
        minWidth: 0,
    },
    header: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        minBlockSize: "3rem",
        paddingInline: space[3],
        borderBlockEndWidth: "1px",
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    title: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        flex: 1,
        minWidth: 0,
    },
    actions: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
    },
});

/** A place of an app, which its entry in the chrome's sidebar leads to. */
export interface ChromePlace {
    /** The identifier, which the current place matches. */
    readonly id: string;
    /** The title. */
    readonly title: string;
    /** The icon. */
    readonly icon: IconName;
    /** How many things wait there, none for nothing. */
    readonly count?: number | undefined;
}

/** The properties of the chrome around an app's screens. */
export interface ChromeProperties {
    /** The app's own places, listed under its entry. */
    readonly places: readonly ChromePlace[];
    /** The place the person is at, none for the app's start. */
    readonly current?: string | undefined;
    /** Go to one of the app's places. */
    readonly onGo: (place: string) => void;
    /** The screen. */
    readonly children: JSX.Element;
}

/** Render the frame every app of a space shares: spaces, apps and search, beside the screen. */
export function Chrome(properties: ChromeProperties): JSX.Element {
    // read the installation origin the page is served at, and the domain below it
    const environment = useEnvironment();
    const locale = useLocale();
    const actions = useActions();
    const page = new URL(environment.navigation.currentEntry.url);
    const domain = page.hostname.slice(page.hostname.indexOf(".") + 1);
    const origin = present(InstallationOrigin.parse(page.hostname, domain), "an app's origin");

    // follow the account's spaces and this space's apps
    const spaces = useQuery(
        accounts.space,
        (query) =>
            query.findMany({ where: { deletedAt: { isNull: true } }, orderBy: { name: "asc" } }),
        { scope: "account" },
    );
    const current = createMemo(() => spaces().find((each) => each.name === origin.space));
    const apps = createSpaceApps(() => current()?.id);
    const home = () => apps().find((app) => app.isHome);
    const href = (app: SpaceApp) => SpaceApp.href(app, page, origin, domain);

    // title a space as the person titled it, else as Home for the account's home space
    const titleOf = (each: { readonly name: string; readonly title: string | null }) =>
        each.title ??
        (each.name === accounts.HOME_SPACE ? locale.render(t.context("title")`Home`) : each.name);
    const title = () => {
        const own = current();

        return own === undefined ? origin.space : titleOf(own);
    };

    return (
        <SidebarProvider scroll="inset">
            <Sidebar collapsible="icon">
                {/* The space, and the others to switch to */}
                <SidebarHeader>
                    <SidebarMenu>
                        <SidebarMenuItem>
                            <DropdownMenu>
                                <DropdownMenuTrigger
                                    render={(attributes) => <SidebarMenuButton {...attributes} />}
                                >
                                    <SpaceMark title={title()} />
                                    <span {...style.attrs(text.sm, styles.space)}>{title()}</span>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="start">
                                    <DropdownMenuLabel>
                                        {locale.render(t.context("title")`Spaces`)}
                                    </DropdownMenuLabel>
                                    <Show when={home()}>
                                        {(own) => (
                                            <For each={spaces()}>
                                                {(each) => (
                                                    <DropdownMenuItem
                                                        render={(attributes) => (
                                                            <a
                                                                {...attributes}
                                                                href={spaceHref(own(), each.name)}
                                                            />
                                                        )}
                                                    >
                                                        <SpaceMark title={titleOf(each)} />
                                                        {titleOf(each)}
                                                    </DropdownMenuItem>
                                                )}
                                            </For>
                                        )}
                                    </Show>
                                </DropdownMenuContent>
                            </DropdownMenu>
                        </SidebarMenuItem>

                        {/* Search across the space, as its shortcut opens it anywhere */}
                        <SidebarMenuItem>
                            <SidebarMenuButton onClick={() => actions.start(palette, null)}>
                                <Icon name="magnifying-glass" />
                                {locale.render(t.context("action")`Search`)}
                            </SidebarMenuButton>
                            <SidebarMenuBadge>
                                <KbdShortcut keys="mod+k" />
                            </SidebarMenuBadge>
                        </SidebarMenuItem>

                        {/* The space's home */}
                        <Show when={home()}>
                            {(each) => (
                                <SidebarMenuItem>
                                    <SidebarMenuButton
                                        href={href(each())}
                                        active={each().name === origin.name}
                                    >
                                        <Icon name="house" />
                                        {locale.render(t.context("title")`Home`)}
                                    </SidebarMenuButton>
                                </SidebarMenuItem>
                            )}
                        </Show>
                    </SidebarMenu>
                </SidebarHeader>

                {/* The space's apps, this one open on its own places */}
                <SidebarContent>
                    <SidebarGroup>
                        <SidebarGroupLabel>
                            {locale.render(t.context("title")`Apps`)}
                        </SidebarGroupLabel>
                        <SidebarMenu>
                            <For each={apps().filter((app) => !app.isHome)}>
                                {(app) => (
                                    <SidebarMenuItem>
                                        <SidebarMenuButton
                                            href={href(app)}
                                            active={
                                                app.name === origin.name &&
                                                properties.current === undefined
                                            }
                                        >
                                            <LazyIcon name={app.icon} />
                                            {locale.render(app.title)}
                                        </SidebarMenuButton>
                                        <Show when={app.name === origin.name}>
                                            <AppPlaces {...properties} />
                                        </Show>
                                    </SidebarMenuItem>
                                )}
                            </For>
                        </SidebarMenu>
                    </SidebarGroup>
                </SidebarContent>

                {/* The account */}
                <SidebarFooter>
                    <div {...style.attrs(text.sm, styles.account)}>
                        <Avatar size="sm">
                            <AvatarFallback>
                                {origin.handle.slice(0, 1).toUpperCase()}
                            </AvatarFallback>
                        </Avatar>
                        <span {...style.attrs(styles.space)}>@{origin.handle}</span>
                    </div>
                </SidebarFooter>
                <SidebarRail />
            </Sidebar>

            {/* The screen */}
            <SidebarInset>{properties.children}</SidebarInset>
        </SidebarProvider>
    );

    /** Write where a space opens: at its home, which this space's home names. */
    function spaceHref(own: SpaceApp, name: string): string {
        return SpaceApp.href(own, page, { ...origin, space: name }, domain);
    }
}

/** Render the bar atop a screen: the sidebar's toggle, where the person is, and what to do. */
export function ChromeHeader(properties: {
    /** Where the person is, such as a breadcrumb. */
    readonly children: JSX.Element;
    /** What the person can do here, such as a button creating something. */
    readonly actions?: JSX.Element;
}): JSX.Element {
    return (
        <header {...style.attrs(text.sm, styles.header)}>
            <SidebarTrigger />
            <div {...style.attrs(styles.title)}>{properties.children}</div>
            <div {...style.attrs(styles.actions)}>{properties.actions}</div>
        </header>
    );
}

/** Render an app's places under its entry in the sidebar. */
function AppPlaces(properties: ChromeProperties): JSX.Element {
    return (
        <SidebarMenuSub>
            <For each={properties.places}>
                {(place) => (
                    <SidebarMenuSubItem>
                        <SidebarMenuSubButton
                            active={place.id === properties.current}
                            onClick={() => properties.onGo(place.id)}
                        >
                            <LazyIcon name={place.icon} />
                            {place.title}
                        </SidebarMenuSubButton>
                        <Show when={place.count}>
                            {(count) => <SidebarMenuBadge>{count()}</SidebarMenuBadge>}
                        </Show>
                    </SidebarMenuSubItem>
                )}
            </For>
        </SidebarMenuSub>
    );
}

/** Render a space's mark: its first letter on the accent. */
function SpaceMark(properties: {
    /** The space's title. */
    readonly title: string;
}): JSX.Element {
    return (
        <span aria-hidden="true" {...style.attrs(text.xs, styles.mark(swatch.accentSolid))}>
            {properties.title.slice(0, 1).toUpperCase()}
        </span>
    );
}

/** Follow a space's apps, read again at each change of its installations. */
function createSpaceApps(spaceId: () => string | undefined): () => readonly SpaceApp[] {
    // follow the view's space's installations, reading its apps again at each change
    const connect = useConnect();
    const installations = useQuery(installation, (query) => query.findMany({}));
    const [apps, setApps] = createSignal<readonly SpaceApp[]>([]);
    createEffect(
        () => [spaceId(), installations()] as const,
        ([id]) => {
            // read the apps from the space's store description, once the space is known
            if (id === undefined) {
                return;
            }
            const store = connect(`${ScopePath.of(id)}${PackageMount.path(stackPackage.id)}`, id);
            SpaceApp.list(store.store).then(setApps, (error: unknown) => {
                globalThis.reportError(error);
            });
        },
    );

    return apps;
}
