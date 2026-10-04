import { Icon } from "@destack/icon";
import type { JSX } from "@solidjs/web";
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

/** Show an app's sidebar of notebooks beside its main content. */
export function SidebarExample(): JSX.Element {
    return (
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
    );
}
