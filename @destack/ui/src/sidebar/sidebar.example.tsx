import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/icon";
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

/** An app's sidebar of notebooks beside its main content. */
export const sidebarNotebooks = defineExample({
    of: Sidebar,
    name: "notebooks",
    description: "an app's sidebar of notebooks beside its main content",
    render: () => (
        <SidebarProvider>
            <Sidebar collapsible="icon">
                <SidebarHeader>Notes</SidebarHeader>
                <SidebarContent>
                    <SidebarGroup>
                        <SidebarGroupLabel>Notebooks</SidebarGroupLabel>
                        <SidebarMenu>
                            <SidebarMenuItem>
                                <SidebarMenuButton href="/trips" isActive>
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
                <SidebarTrigger />
                <main>Trips</main>
            </SidebarInset>
        </SidebarProvider>
    ),
});

/** An app's sidebar of notebooks collapsed to its icons beside its main content. */
export const sidebarNotebooksCollapsed = defineExample({
    of: Sidebar,
    name: "notebooks-collapsed",
    description: "an app's sidebar of notebooks collapsed to its icons beside its main content",
    render: () => (
        <SidebarProvider defaultOpen={false}>
            <Sidebar collapsible="icon">
                <SidebarHeader>Notes</SidebarHeader>
                <SidebarContent>
                    <SidebarGroup>
                        <SidebarGroupLabel>Notebooks</SidebarGroupLabel>
                        <SidebarMenu>
                            <SidebarMenuItem>
                                <SidebarMenuButton href="/trips" isActive>
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
                <SidebarTrigger />
                <main>Trips</main>
            </SidebarInset>
        </SidebarProvider>
    ),
});
