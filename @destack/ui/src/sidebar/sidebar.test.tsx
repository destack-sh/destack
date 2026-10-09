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
import { classes, render, stubPopovers } from "@destack/view/test";

/** Render a page with a sidebar of notebooks, collapsing to its icons. */
function drawPage(changes: boolean[]): HTMLElement {
    onTestFinished(() => localStorage.clear());

    return render(() => (
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
    )).container;
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
    const { container } = render(() => (
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

test("keep a sidebar that cannot collapse as a column on narrow screens, where others turn into sheets", async () => {
    // answer every media query as a phone's narrow screen does, for this test alone
    const original = window.matchMedia;
    window.matchMedia = (query: string) => {
        const list = original.call(window, query);
        Object.defineProperty(list, "matches", { value: true });

        return list;
    };
    onTestFinished(() => {
        window.matchMedia = original;
    });

    // render one sidebar that cannot collapse and one that collapses off canvas
    const { container } = render(() => (
        <>
            <SidebarProvider>
                <Sidebar collapsible="none">
                    <SidebarContent>Settings</SidebarContent>
                </Sidebar>
            </SidebarProvider>
            <SidebarProvider>
                <Sidebar collapsible="offcanvas">
                    <SidebarContent>Notebooks</SidebarContent>
                </Sidebar>
            </SidebarProvider>
        </>
    ));
    await new Promise((resolve) => {
        setTimeout(resolve, 0);
    });
    flush();

    // show the settings column in place, and the notebooks as the phone's sheet titled for assistive technology
    expect(
        [...container.querySelectorAll("[data-slot=sidebar]")].map((sidebar) => [
            sidebar.textContent,
            sidebar.getAttribute("data-mobile"),
        ]),
    ).toEqual([
        ["Settings", null],
        ["SidebarNotebooks", "true"],
    ]);
});
