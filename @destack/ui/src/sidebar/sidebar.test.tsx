import { expect, onTestFinished, test } from "@destack/test";
import { flush } from "@destack/view";
import {
    Sidebar,
    SidebarContent,
    SidebarGroup,
    SidebarGroupLabel,
    SidebarInset,
    SidebarMenu,
    SidebarMenuButton,
    SidebarMenuItem,
    SidebarMenuSkeleton,
    SidebarProvider,
    SidebarTrigger,
} from "./index.ts";
import { classes, draw, stubPopovers } from "@destack/view/test";

/** Render a page with a sidebar of notebooks, collapsing to its icons. */
function drawPage(changes: boolean[]): HTMLElement {
    onTestFinished(() => localStorage.clear());

    return draw(() => (
        <SidebarProvider onOpenChange={(open) => changes.push(open)}>
            <Sidebar collapsible="icon">
                <SidebarContent>
                    <SidebarGroup>
                        <SidebarGroupLabel>Notebooks</SidebarGroupLabel>
                        <SidebarMenu>
                            <SidebarMenuItem>
                                <SidebarMenuButton href="/trips" active tooltip="Trips">
                                    Trips
                                </SidebarMenuButton>
                            </SidebarMenuItem>
                            <SidebarMenuItem>
                                <SidebarMenuButton>New notebook</SidebarMenuButton>
                            </SidebarMenuItem>
                        </SidebarMenu>
                    </SidebarGroup>
                </SidebarContent>
            </Sidebar>
            <SidebarInset>
                <SidebarTrigger />
            </SidebarInset>
        </SidebarProvider>
    ));
}

test("collapse the sidebar from its trigger, reporting the change", () => {
    const changes: boolean[] = [];
    const container = drawPage(changes);
    const sidebar = container.querySelector("[data-slot=sidebar]");
    const trigger = container.querySelector<HTMLElement>("[data-slot=sidebar-trigger]");
    const expanded = [
        sidebar?.getAttribute("data-state"),
        trigger?.getAttribute("aria-expanded"),
        trigger?.getAttribute("aria-controls") === sidebar?.id,
    ];
    const width = classes(sidebar?.parentElement ?? container)[0];
    trigger?.click();
    flush();
    const collapsed = [
        sidebar?.getAttribute("data-state"),
        sidebar?.getAttribute("data-collapsible"),
        trigger?.getAttribute("aria-expanded"),
    ];
    const isNarrower = classes(sidebar?.parentElement ?? container)[0] !== width;
    expect([expanded, collapsed, isNarrower, changes]).toEqual([
        ["expanded", "true", true],
        ["collapsed", "icon", "false"],
        true,
        [false],
    ]);
});

test("render a menu button as a link to the current page with href, else as a button", () => {
    const container = drawPage([]);
    const buttons = [...container.querySelectorAll("[data-slot=sidebar-menu-button]")].map(
        (button) =>
            [
                button.tagName,
                button.getAttribute("aria-current"),
                button.getAttribute("data-active"),
            ].join(" "),
    );
    expect(buttons).toEqual(["A page true", "BUTTON  "]);
});

test("start as the viewer left the sidebar, remembering each change", () => {
    localStorage.setItem("destack-sidebar-open", "false");
    const container = drawPage([]);
    const sidebar = container.querySelector("[data-slot=sidebar]");
    const started = sidebar?.getAttribute("data-state");
    container.querySelector<HTMLElement>("[data-slot=sidebar-trigger]")?.click();
    flush();
    expect([started, localStorage.getItem("destack-sidebar-open")]).toEqual(["collapsed", "true"]);
});

test("name a menu button in a tooltip only while the sidebar shows its icons", () => {
    stubPopovers();
    const container = drawPage([]);
    const link = container.querySelector<HTMLElement>("a[data-slot=sidebar-menu-button]");
    const tooltip = container.querySelector("[role=tooltip]");
    link?.dispatchEvent(new Event("pointerenter"));
    const expanded = [
        link?.hasAttribute("aria-describedby"),
        tooltip?.hasAttribute("data-popover-open"),
    ];
    container.querySelector<HTMLElement>("[data-slot=sidebar-trigger]")?.click();
    flush();
    link?.dispatchEvent(new Event("pointerenter"));
    flush();
    expect([
        expanded,
        link?.getAttribute("aria-describedby") === tooltip?.id,
        tooltip?.hasAttribute("data-popover-open"),
        tooltip?.textContent,
    ]).toEqual([[false, false], true, true, "Trips"]);
});

test("show placeholder rows through the shared skeleton while entries load", () => {
    const container = draw(() => (
        <SidebarProvider>
            <SidebarMenuSkeleton showIcon />
        </SidebarProvider>
    ));
    expect(
        [...container.querySelectorAll("[data-slot=skeleton]")].map((skeleton) =>
            skeleton.getAttribute("data-sidebar"),
        ),
    ).toEqual(["menu-skeleton-icon", "menu-skeleton-text"]);
});
