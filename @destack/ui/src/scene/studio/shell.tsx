import type { IconName } from "@destack/icon";
import type { PlainDate } from "@destack/schema";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, space, swatch, weight } from "@destack/theme/tokens.stylex";
import { For, type JSX, palette, Show, useActions, useLocale } from "@destack/view";
import { Avatar, AvatarFallback, AvatarImage } from "../../avatar/index.ts";
import { Cluster } from "../../cluster/index.ts";
import { Icon, LazyIcon } from "@destack/ui/icon";
import { KbdShortcut } from "../../kbd/index.ts";
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
} from "../../sidebar/index.ts";
import { type Person, personOf } from "./state.ts";

/** The styles of the studio's shell. */
const styles = style.create({
    brand: {
        fontWeight: weight.medium,
    },
    mark: {
        width: space[5],
        height: space[5],
        borderRadius: radius[2],
        backgroundColor: swatch.accentSolid,
    },
    due: {
        flexShrink: 0,
        color: color.mutedForeground,
        whiteSpace: "nowrap",
    },
    late: {
        color: swatch.destructiveText,
    },
});

/** A place the sidebar leads to. */
export interface Place {
    /** The id. */
    readonly id: string;
    /** The name. */
    readonly name: string;
    /** The icon. */
    readonly icon: IconName;
    /** How many things wait there, when any do. */
    readonly count?: number | undefined;
}

/** A group of places under a label. */
export interface PlaceGroup {
    /** The label. */
    readonly label: string;
    /** The places. */
    readonly places: readonly Place[];
}

/** Render a person's picture, or their initials without one. */
export function Face(properties: {
    /** The person. */
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

/** Render the day a task is due, marked once it is late. */
export function Due(properties: {
    /** The day. */
    readonly day: PlainDate;
    /** Whether it passed. */
    readonly isLate: boolean;
}): JSX.Element {
    const locale = useLocale();

    return (
        <span {...style.attrs(styles.due, properties.isLate && styles.late)}>
            {locale.plainDate(properties.day, { month: "short", day: "numeric" })}
        </span>
    );
}

/** Render the studio's shell: its sidebar of places, and the screen beside it. */
export function Shell(properties: {
    /** The groups of places. */
    readonly groups: readonly PlaceGroup[];
    /** The place the person is at. */
    readonly current: string;
    /** Go to a place. */
    readonly onGo: (place: string) => void;
    /** The screen. */
    readonly children: JSX.Element;
}): JSX.Element {
    const actions = useActions();
    const me = personOf("ada");

    return (
        <SidebarProvider scroll="inset">
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
                            <SidebarMenuButton onClick={() => actions.start(palette, null)}>
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
                                                    active={place.id === properties.current}
                                                    onClick={() => properties.onGo(place.id)}
                                                >
                                                    <LazyIcon name={place.icon} />
                                                    {place.name}
                                                </SidebarMenuButton>
                                                <Show when={place.count}>
                                                    {(count) => (
                                                        <SidebarMenuBadge>
                                                            {count()}
                                                        </SidebarMenuBadge>
                                                    )}
                                                </Show>
                                            </SidebarMenuItem>
                                        )}
                                    </For>
                                </SidebarMenu>
                            </SidebarGroup>
                        )}
                    </For>
                </SidebarContent>
                <SidebarFooter>
                    <Show when={me}>
                        {(person) => (
                            <Cluster space="2" align="center" xstyle={text.sm}>
                                <Face person={person()} />
                                {person().name}
                            </Cluster>
                        )}
                    </Show>
                </SidebarFooter>
                <SidebarRail />
            </Sidebar>
            <SidebarInset>{properties.children}</SidebarInset>
        </SidebarProvider>
    );
}
