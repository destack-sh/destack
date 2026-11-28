import { SpaceAddress } from "@destack/account/address";
import type { IconName } from "@destack/icon";
import homeIcon from "@destack/icon/phosphor/house";
import searchIcon from "@destack/icon/phosphor/magnifying-glass";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, space, swatch, weight } from "@destack/theme/tokens.stylex";
import {
    For,
    type JSX,
    palette,
    Show,
    useActions,
    useEnvironment,
    useLocale,
    useView,
} from "@destack/view";
import { ViewApp } from "@destack/view/declare";
import { Avatar, AvatarFallback } from "../avatar/index.ts";
import { Icon, LazyIcon } from "../icon/index.ts";
import { KbdShortcut } from "../kbd/index.ts";
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
} from "../sidebar/index.ts";

/** The styles of the chrome: the space's mark and title, the account, and the header bar. */
const styles = style.create({
    mark: {
        display: "grid",
        placeItems: "center",
        flexShrink: 0,
        width: space[5],
        height: space[5],
        borderRadius: radius[2],
        backgroundColor: swatch.accentSolid,
        color: swatch.accentLabel,
        fontWeight: weight.semibold,
    },
    title: {
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
    where: {
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

/** An app's own places, the one the person is at, and the way to go to another. */
export interface ChromePlaces {
    /** The places. */
    readonly items: readonly ChromePlace[];
    /** The place the person is at, none for none of them, such as an object's page. */
    readonly current: string | undefined;
    /** Go to a place. */
    readonly onGo: (place: string) => void;
}

/** The properties of the chrome around an app's screens. */
export interface ChromeProperties {
    /** The app's own places, listed under its entry, none for an app without any. */
    readonly places?: ChromePlaces | undefined;
    /** The screen. */
    readonly children: JSX.Element;
}

/** Render the frame every app of a space shares: its apps and search, beside the screen. */
export function Chrome(properties: ChromeProperties): JSX.Element {
    // read the space's apps from the host, this app among them
    const locale = useLocale();
    const actions = useActions();
    const context = useView();
    const page = new URL(useEnvironment().navigation.currentEntry.url);
    const home = () => context.apps.find((app) => app.isHome);
    const isOwn = (app: ViewApp) => app.installation === context.installation;
    const href = (app: ViewApp) => ViewApp.href(app, context.spaceAddress, page);

    // title the person's own space as Personal, apart from the Home app, any other by its title
    const title =
        context.home === context.space
            ? locale.render(t.context("title")`Personal`)
            : context.spaceTitle;
    const { handle } = SpaceAddress.origin(context.spaceAddress);

    return (
        <SidebarProvider scroll="inset">
            <Sidebar collapsible="icon">
                {/* The space, search and home */}
                <SidebarHeader>
                    <SidebarMenu>
                        <SidebarMenuItem>
                            <div {...style.attrs(text.sm, styles.account)}>
                                <span aria-hidden="true" {...style.attrs(text.xs, styles.mark)}>
                                    {title.slice(0, 1).toUpperCase()}
                                </span>
                                <span {...style.attrs(styles.title)}>{title}</span>
                            </div>
                        </SidebarMenuItem>
                        <SidebarMenuItem>
                            <SidebarMenuButton onClick={() => actions.start(palette, null)}>
                                <Icon icon={searchIcon} />
                                {locale.render(t.context("action")`Search`)}
                            </SidebarMenuButton>
                            <SidebarMenuBadge>
                                <KbdShortcut keys="mod+k" />
                            </SidebarMenuBadge>
                        </SidebarMenuItem>
                        <Show when={home()}>
                            {(own) => (
                                <SidebarMenuItem>
                                    <SidebarMenuButton href={href(own())} active={isOwn(own())}>
                                        <Icon icon={homeIcon} />
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
                            <For each={context.apps.filter((app) => !app.isHome)}>
                                {(app) => (
                                    <SidebarMenuItem>
                                        <SidebarMenuButton
                                            href={href(app)}
                                            active={
                                                isOwn(app) &&
                                                properties.places?.current === undefined
                                            }
                                        >
                                            <LazyIcon name={app.icon} />
                                            {locale.render(app.title)}
                                        </SidebarMenuButton>
                                        <Show when={isOwn(app) ? properties.places : undefined}>
                                            {(places) => <AppPlaces places={places()} />}
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
                            <AvatarFallback>{handle.slice(0, 1).toUpperCase()}</AvatarFallback>
                        </Avatar>
                        <span {...style.attrs(styles.title)}>@{handle}</span>
                    </div>
                </SidebarFooter>
                <SidebarRail />
            </Sidebar>

            {/* The screen */}
            <SidebarInset>{properties.children}</SidebarInset>
        </SidebarProvider>
    );
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
            <div {...style.attrs(styles.where)}>{properties.children}</div>
            <div {...style.attrs(styles.actions)}>{properties.actions}</div>
        </header>
    );
}

/** Render an app's places under its entry in the sidebar. */
function AppPlaces(properties: {
    /** The app's places. */
    readonly places: ChromePlaces;
}): JSX.Element {
    return (
        <SidebarMenuSub>
            <For each={properties.places.items}>
                {(place) => (
                    <SidebarMenuSubItem>
                        <SidebarMenuSubButton
                            active={place.id === properties.places.current}
                            onClick={() => properties.places.onGo(place.id)}
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
