import type { BrowserEnvironment } from "@destack/environment/browser";
import { BrowserSimulation } from "@destack/environment/browser/simulation";
import { Simulation } from "@destack/environment/simulation";
import { expect, onTestFinished, test } from "@destack/test";
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
import { flush } from "@destack/view";
import { classes, render, stubPopovers } from "@destack/view/test";

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** Open a tab whose storage starts empty, removing its world once the test ends. */
function openTab(): BrowserEnvironment {
    const simulation = new Simulation({ seed: 1, start: START });
    onTestFinished(() => simulation[Symbol.asyncDispose]());

    return new BrowserSimulation(simulation).environment("tab");
}

/** Render a page with a sidebar of notebooks, collapsing to its icons, in a tab. */
function drawPage(changes: boolean[], tab: BrowserEnvironment = openTab()): HTMLElement {
    return render(
        () => (
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
        ),
        { environment: tab },
    ).container;
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
    const tab = openTab();
    tab.storage.setItem("destack-sidebar-open", "false");
    const container = drawPage([], tab);
    const sidebar = container.querySelector("[data-slot=sidebar]");
    const started = sidebar?.getAttribute("data-state");
    container.querySelector<HTMLElement>("[data-slot=sidebar-trigger]")?.click();
    flush();
    expect([started, tab.storage.getItem("destack-sidebar-open")]).toEqual(["collapsed", "true"]);
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
    // render in a tab whose display matches the phone's narrow screen query
    await using simulation = new Simulation({ seed: 1, start: START });
    const browsers = new BrowserSimulation(simulation, {
        media: { "(max-width: 47.999rem)": true },
    });
    const environment = browsers.environment("phone");

    // render one sidebar that cannot collapse and one that collapses off canvas
    const { container } = render(
        () => (
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
        ),
        { environment },
    );
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
