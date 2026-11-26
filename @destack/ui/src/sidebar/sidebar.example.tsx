import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { Icon } from "@destack/ui/icon";
import { Separator } from "../separator/index.ts";
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
} from "./sidebar.tsx";

/** The header and page of the inset beside the sidebar, after shadcn/ui's sidebar demo. */
const styles = style.create({
    header: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        height: size[4],
        paddingInline: space[4],
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    separator: {
        alignSelf: "center",
        height: space[4],
    },
    title: {
        fontWeight: weight.medium,
    },
    page: {
        padding: space[4],
        color: color.mutedForeground,
    },
});
/** An app's sidebar of notebooks beside its main content. */
export const sidebarNotebooks = defineExample({
    of: Sidebar,
    name: "notebooks",
    description: t`An app's sidebar of notebooks beside its main content.`,
    layout: "fullscreen",
    properties: {
        collapsible: "icon",
    },
    render: (properties) => (
        <SidebarProvider>
            <Sidebar {...properties}>
                <SidebarHeader>Notes</SidebarHeader>
                <SidebarContent>
                    <SidebarGroup>
                        <SidebarGroupLabel>Notebooks</SidebarGroupLabel>
                        <SidebarMenu>
                            <SidebarMenuItem>
                                <SidebarMenuButton href="/trips" active>
                                    <Icon name="notebook" />
                                    Trips
                                </SidebarMenuButton>
                                <SidebarMenuBadge>12</SidebarMenuBadge>
                            </SidebarMenuItem>
                        </SidebarMenu>
                    </SidebarGroup>
                </SidebarContent>
                <SidebarFooter>Ada Lovelace</SidebarFooter>
                <SidebarRail />
            </Sidebar>
            <SidebarInset>
                <header {...style.attrs(styles.header)}>
                    <SidebarTrigger />
                    <Separator orientation="vertical" xstyle={styles.separator} />
                    <span {...style.attrs(text.sm, styles.title)}>Trips</span>
                </header>
                <div {...style.attrs(text.sm, styles.page)}>
                    12 notes from Lisbon, Porto and Madeira.
                </div>
            </SidebarInset>
        </SidebarProvider>
    ),
});

/** An app's sidebar of notebooks collapsed to its icons beside its main content. */
export const sidebarNotebooksCollapsed = defineExample({
    of: Sidebar,
    name: "notebooksCollapsed",
    description: t`An app's sidebar of notebooks collapsed to its icons beside its main content.`,
    layout: "fullscreen",
    properties: {
        collapsible: "icon",
    },
    render: (properties) => (
        <SidebarProvider defaultOpen={false}>
            <Sidebar {...properties}>
                <SidebarHeader>Notes</SidebarHeader>
                <SidebarContent>
                    <SidebarGroup>
                        <SidebarGroupLabel>Notebooks</SidebarGroupLabel>
                        <SidebarMenu>
                            <SidebarMenuItem>
                                <SidebarMenuButton href="/trips" active>
                                    <Icon name="notebook" />
                                    Trips
                                </SidebarMenuButton>
                                <SidebarMenuBadge>12</SidebarMenuBadge>
                            </SidebarMenuItem>
                        </SidebarMenu>
                    </SidebarGroup>
                </SidebarContent>
                <SidebarFooter>Ada Lovelace</SidebarFooter>
                <SidebarRail />
            </Sidebar>
            <SidebarInset>
                <header {...style.attrs(styles.header)}>
                    <SidebarTrigger />
                    <Separator orientation="vertical" xstyle={styles.separator} />
                    <span {...style.attrs(text.sm, styles.title)}>Trips</span>
                </header>
                <div {...style.attrs(text.sm, styles.page)}>
                    12 notes from Lisbon, Porto and Madeira.
                </div>
            </SidebarInset>
        </SidebarProvider>
    ),
});
