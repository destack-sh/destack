import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, size, space, stroke, swatch, weight } from "@destack/theme/tokens.stylex";
import { Icon } from "@destack/ui/icon";
import { For, type JSX, Show } from "@destack/view";
import { Avatar, AvatarFallback, AvatarImage } from "../avatar/index.ts";
import {
    Breadcrumb,
    BreadcrumbItem,
    BreadcrumbLink,
    BreadcrumbList,
    BreadcrumbPage,
    BreadcrumbSeparator,
} from "../breadcrumb/index.ts";
import { Cluster } from "../cluster/index.ts";
import { PICTURES } from "../example/picture.ts";
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
    SidebarProvider,
    SidebarRail,
    SidebarTrigger,
} from "../sidebar/index.ts";

/** A place an app's sidebar leads to. */
interface Place {
    /** Its name. */
    readonly name: string;
    /** The icon that stands for it. */
    readonly icon: JSX.Element;
}

/** A group of places under one label in an app's sidebar. */
interface PlaceGroup {
    /** The label over the places. */
    readonly label: string;
    /** The places. */
    readonly places: readonly Place[];
}

/** A person tasks are assigned to. */
interface Person {
    /** Their name. */
    readonly name: string;
    /** Their initials. */
    readonly initials: string;
    /** Their picture, when they have one. */
    readonly picture?: string;
}

/** The people of the studio. */
export const PEOPLE = {
    ada: { name: "Ada Lovelace", initials: "AL", picture: PICTURES.ada },
    jonas: { name: "Jonas Abiola", initials: "JA" },
} as const satisfies Record<string, Person>;

/** The styles of the app: its bar, its page and its sidebar. */
const styles = style.create({
    bar: {
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        flexWrap: "wrap",
        columnGap: space[3],
        rowGap: space[1],
        minHeight: size[4],
        paddingBlock: space[1],
        paddingInline: space[4],
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    grow: {
        flexGrow: 1,
    },
    page: {
        display: "flex",
        flexDirection: "column",
        flexGrow: 1,
        minHeight: 0,
        overflow: "auto",
        padding: space[5],
    },
    brand: {
        fontWeight: weight.medium,
    },
    mark: {
        width: space[5],
        height: space[5],
        borderRadius: radius[2],
        backgroundColor: swatch.accentSolid,
    },
    person: {
        minWidth: 0,
    },
});

/** Render a person's picture, or their initials without one. */
export function Face(properties: {
    /** The person the face shows. */
    readonly person: Person;
}): JSX.Element {
    return (
        <Avatar size="sm">
            <Show when={properties.person.picture}>
                {(picture) => <AvatarImage src={picture()} alt={properties.person.name} />}
            </Show>
            <AvatarFallback>{properties.person.initials}</AvatarFallback>
        </Avatar>
    );
}

/** Render an app: its sidebar of places, then a bar and a page beside it. */
export function App(properties: {
    /** The app's name. */
    readonly name: string;
    /** The groups of places its sidebar leads to. */
    readonly groups: readonly PlaceGroup[];
    /** The place the person is at. */
    readonly current: string;
    /** What follows the breadcrumb in the bar. */
    readonly actions?: JSX.Element;
    /** The page. */
    readonly children: JSX.Element;
}): JSX.Element {
    return (
        <SidebarProvider>
            <Sidebar collapsible="icon">
                <SidebarHeader>
                    <Cluster
                        space="2"
                        align="center"
                        xstyle={[text.sm, styles.brand]}
                        render={(attributes) => <span {...attributes} />}
                    >
                        <span aria-hidden="true" {...style.attrs(styles.mark)} />
                        Harbour Studio
                    </Cluster>
                    <SidebarMenu>
                        <SidebarMenuItem>
                            <SidebarMenuButton>
                                <Icon name="magnifying-glass" />
                                Search
                            </SidebarMenuButton>
                            <SidebarMenuBadge>
                                <KbdShortcut keys="mod+k" />
                            </SidebarMenuBadge>
                        </SidebarMenuItem>
                    </SidebarMenu>
                </SidebarHeader>
                <SidebarContent>
                    <For each={properties.groups}>
                        {(group) => (
                            <SidebarGroup>
                                <SidebarGroupLabel>{group.label}</SidebarGroupLabel>
                                <SidebarMenu>
                                    <For each={group.places}>
                                        {(place) => (
                                            <SidebarMenuItem>
                                                <SidebarMenuButton
                                                    href={`#${place.name}`}
                                                    active={place.name === properties.current}
                                                >
                                                    {place.icon}
                                                    {place.name}
                                                </SidebarMenuButton>
                                            </SidebarMenuItem>
                                        )}
                                    </For>
                                </SidebarMenu>
                            </SidebarGroup>
                        )}
                    </For>
                </SidebarContent>
                <SidebarFooter>
                    <Cluster
                        space="2"
                        align="center"
                        xstyle={[text.sm, styles.person]}
                        render={(attributes) => <span {...attributes} />}
                    >
                        <Face person={PEOPLE.ada} />
                        {PEOPLE.ada.name}
                    </Cluster>
                </SidebarFooter>
                <SidebarRail />
            </Sidebar>
            <SidebarInset>
                <header {...style.attrs(text.sm, styles.bar)}>
                    <SidebarTrigger />
                    <Breadcrumb>
                        <BreadcrumbList>
                            <BreadcrumbItem>
                                <BreadcrumbLink href="#">{properties.name}</BreadcrumbLink>
                            </BreadcrumbItem>
                            <BreadcrumbSeparator />
                            <BreadcrumbItem>
                                <BreadcrumbPage>{properties.current}</BreadcrumbPage>
                            </BreadcrumbItem>
                        </BreadcrumbList>
                    </Breadcrumb>
                    <span {...style.attrs(styles.grow)} />
                    {properties.actions}
                </header>
                <div {...style.attrs(text.sm, styles.page)}>{properties.children}</div>
            </SidebarInset>
        </SidebarProvider>
    );
}
