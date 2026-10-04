import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import {
    NavigationMenu,
    NavigationMenuContent,
    NavigationMenuIndicator,
    NavigationMenuItem,
    NavigationMenuLink,
    NavigationMenuList,
    NavigationMenuTrigger,
} from "./index.ts";
import { draw, focused, press, wait } from "@destack/view/test";

/** Render a site navigation with a disclosed panel and a plain link. */
function drawSite(): HTMLElement {
    return draw(() => (
        <NavigationMenu aria-label="Main">
            <NavigationMenuList>
                <NavigationMenuItem>
                    <NavigationMenuTrigger>Products</NavigationMenuTrigger>
                    <NavigationMenuContent>
                        <NavigationMenuLink href="/notes">Notes</NavigationMenuLink>
                        <NavigationMenuLink href="/tasks">Tasks</NavigationMenuLink>
                    </NavigationMenuContent>
                </NavigationMenuItem>
                <NavigationMenuItem>
                    <NavigationMenuLink href="/pricing" active>
                        Pricing
                    </NavigationMenuLink>
                </NavigationMenuItem>
            </NavigationMenuList>
        </NavigationMenu>
    ));
}

/** Render a navigation with two panels, an indicator and short hover delays. */
function drawPanels(): HTMLElement {
    return draw(() => (
        <NavigationMenu aria-label="Main" delayDuration={20} skipDelayDuration={60}>
            <NavigationMenuList>
                <NavigationMenuItem>
                    <NavigationMenuTrigger>Products</NavigationMenuTrigger>
                    <NavigationMenuContent>
                        <NavigationMenuLink href="/notes">Notes</NavigationMenuLink>
                        <NavigationMenuLink href="/tasks">Tasks</NavigationMenuLink>
                    </NavigationMenuContent>
                </NavigationMenuItem>
                <NavigationMenuItem>
                    <NavigationMenuTrigger>Company</NavigationMenuTrigger>
                    <NavigationMenuContent>
                        <NavigationMenuLink href="/about">About</NavigationMenuLink>
                    </NavigationMenuContent>
                </NavigationMenuItem>
                <NavigationMenuIndicator />
            </NavigationMenuList>
        </NavigationMenu>
    ));
}

/** Read which trigger is expanded, by its text. */
function expanded(container: Element): string {
    return container.querySelector("[aria-expanded=true]")?.textContent ?? "none";
}

test("disclose a navigation menu's panel from its trigger and close it on Escape", () => {
    const container = drawSite();
    const trigger = container.querySelector<HTMLElement>("[data-slot=navigation-menu-trigger]");
    trigger?.click();
    flush();
    const opened = [
        trigger?.getAttribute("aria-expanded"),
        container.querySelector("[data-slot=navigation-menu-content]")?.hasAttribute("hidden"),
    ];
    container.querySelector<HTMLElement>("[href='/tasks']")?.focus();
    press("Escape");
    flush();

    // the panel hides and the focus returns to its trigger
    expect([opened, trigger?.getAttribute("aria-expanded"), focused()]).toEqual([
        ["true", false],
        "false",
        "Products",
    ]);
});

test("move between a navigation menu's top-level entries with left and right, leaving the panel's links out", () => {
    const container = drawSite();
    container.querySelector<HTMLElement>("[data-slot=navigation-menu-trigger]")?.focus();
    const visited = ["ArrowRight", "ArrowRight", "ArrowLeft"].map((key) => {
        press(key);

        return focused();
    });
    expect(visited).toEqual(["Pricing", "Products", "Pricing"]);
    expect(container.querySelector("[aria-current=page]")?.textContent).toBe("Pricing");
});

test("render the open panel inside the viewport below the list, with the indicator shown", () => {
    const container = drawPanels();
    container.querySelector<HTMLElement>("[data-slot=navigation-menu-trigger]")?.click();
    flush();
    const viewport = container.querySelector("[data-slot=navigation-menu-viewport]");
    const shown = [
        ...(viewport?.querySelectorAll("[data-slot=navigation-menu-content]:not([hidden])") ?? []),
    ];
    expect([
        viewport?.hasAttribute("hidden"),
        shown.map((panel) => panel.textContent),
        container.querySelector("[data-slot=navigation-menu-indicator]")?.hasAttribute("hidden"),
    ]).toEqual([false, ["NotesTasks"], false]);
});

test("open a hovered trigger after the delay, and the next one at once while a panel is open", async () => {
    const container = drawPanels();
    const [products, company] = container.querySelectorAll("[data-slot=navigation-menu-trigger]");
    products?.dispatchEvent(new Event("pointerenter"));
    const early = expanded(container);
    await wait(40);
    const late = expanded(container);
    products?.dispatchEvent(new Event("pointerleave"));
    company?.dispatchEvent(new Event("pointerenter"));
    flush();
    expect([early, late, expanded(container)]).toEqual(["none", "Products", "Company"]);
});

test("carry Tab from an open trigger into its panel and from the panel's last link on to the next trigger", () => {
    const container = drawPanels();
    const trigger = container.querySelector<HTMLElement>("[data-slot=navigation-menu-trigger]");
    trigger?.click();
    flush();
    trigger?.focus();
    press("Tab");
    const inside = focused();
    container.querySelector<HTMLElement>("[href='/tasks']")?.focus();
    press("Tab");
    expect([inside, focused()]).toEqual(["Notes", "Company"]);
});
