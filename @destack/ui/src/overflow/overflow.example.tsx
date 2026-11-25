import textB from "@destack/icon/phosphor/text-b";
import textItalic from "@destack/icon/phosphor/text-italic";
import textUnderline from "@destack/icon/phosphor/text-underline";
import listBullets from "@destack/icon/phosphor/list-bullets";
import listNumbers from "@destack/icon/phosphor/list-numbers";
import link from "@destack/icon/phosphor/link";
import image from "@destack/icon/phosphor/image";
import code from "@destack/icon/phosphor/code";
import { defineExample, type PropertiesOf } from "@destack/package/declare";
import { For, type JSX } from "@destack/view";
import {
    Breadcrumb,
    BreadcrumbItem,
    BreadcrumbLink,
    BreadcrumbList,
    BreadcrumbPage,
    BreadcrumbSeparator,
} from "../breadcrumb/index.ts";
import { Badge } from "../badge/index.ts";
import { Button } from "../button/index.ts";
import { DropdownMenuItem } from "../dropdown-menu/index.ts";
import { Icon } from "../icon/index.ts";
import {
    Menubar,
    MenubarContent,
    MenubarItem,
    MenubarMenu,
    MenubarTrigger,
} from "../menubar/index.ts";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../tabs/index.ts";
import { Toolbar, ToolbarButton, ToolbarSeparator } from "../toolbar/index.ts";
import {
    Overflow,
    OverflowItem,
    OverflowMenu,
    OverflowMenuContent,
    OverflowMenuTrigger,
} from "./index.ts";

/** The views of a project. */
const VIEWS = ["Overview", "Roadmap", "Issues", "Releases", "Members", "Billing", "Settings"];

/** A project's views, the ones that do not fit in a menu. */
export const overflowTabs = defineExample({
    of: Overflow,
    name: "tabs",
    description: "a project's views as tabs, the ones that do not fit in a menu",
    render: () => (
        <Tabs defaultValue="Overview">
            <Overflow>
                <TabsList aria-label="Project" variant="line">
                    <For each={VIEWS}>
                        {(view) => <TabsTrigger value={view}>{view}</TabsTrigger>}
                    </For>
                </TabsList>
                <OverflowMenu />
            </Overflow>
            <For each={VIEWS}>{(view) => <TabsContent value={view}>{view}</TabsContent>}</For>
        </Tabs>
    ),
});

/** A project's views in a narrow column. */
export const overflowNarrow = defineExample({
    of: Overflow,
    name: "narrow",
    description: "a project's views in a column too narrow for them",
    render: () => (
        <div style={{ width: "320px" }}>
            <Tabs defaultValue="Overview">
                <Overflow>
                    <TabsList aria-label="Project" variant="line">
                        <For each={VIEWS}>
                            {(view) => <TabsTrigger value={view}>{view}</TabsTrigger>}
                        </For>
                    </TabsList>
                    <OverflowMenu />
                </Overflow>
                <For each={VIEWS}>{(view) => <TabsContent value={view}>{view}</TabsContent>}</For>
            </Tabs>
        </div>
    ),
});

/** The pages above a page, the middle ones in a menu where they do not fit. */
export const overflowBreadcrumb = defineExample({
    of: Overflow,
    name: "breadcrumb",
    description: "the pages above a page, the ones after the first in a menu",
    render: () => (
        <Breadcrumb>
            <Overflow>
                <BreadcrumbList>
                    <BreadcrumbItem>
                        <BreadcrumbLink href="#home">Home</BreadcrumbLink>
                    </BreadcrumbItem>
                    <BreadcrumbSeparator />
                    <BreadcrumbItem>
                        <OverflowMenu>
                            <OverflowMenuTrigger size="icon-sm" />
                            <OverflowMenuContent />
                        </OverflowMenu>
                    </BreadcrumbItem>
                    <For each={["Notebooks", "Trips", "Portugal", "Lisbon"]}>
                        {(page) => (
                            <>
                                <BreadcrumbSeparator />
                                <BreadcrumbItem>
                                    <BreadcrumbLink href={`#${page}`}>{page}</BreadcrumbLink>
                                </BreadcrumbItem>
                            </>
                        )}
                    </For>
                    <BreadcrumbSeparator />
                    <BreadcrumbItem>
                        <BreadcrumbPage>Day two</BreadcrumbPage>
                    </BreadcrumbItem>
                </BreadcrumbList>
            </Overflow>
        </Breadcrumb>
    ),
});

/** What a toolbar's buttons do, the last groups hiding first. */
const COMMANDS = [
    [
        ["Bold", textB],
        ["Italic", textItalic],
        ["Underline", textUnderline],
    ],
    [
        ["Bullets", listBullets],
        ["Numbers", listNumbers],
    ],
    [
        ["Link", link],
        ["Image", image],
        ["Code", code],
    ],
] as const;

/** A toolbar whose buttons hide by their priority. */
export const overflowToolbar = defineExample({
    of: Overflow,
    name: "toolbar",
    description: "a toolbar whose buttons hide where they do not fit, its send button never",
    render: () => (
        <Toolbar aria-label="Format">
            <Overflow>
                <For each={COMMANDS}>
                    {(group, index) => (
                        <>
                            {index() > 0 ? <ToolbarSeparator /> : undefined}
                            <For each={group}>
                                {([label, icon]) => (
                                    <ToolbarButton aria-label={label}>
                                        <Icon icon={icon} />
                                    </ToolbarButton>
                                )}
                            </For>
                        </>
                    )}
                </For>
                <OverflowMenu />
                <OverflowItem priority={Infinity}>
                    <Button>Send</Button>
                </OverflowItem>
            </Overflow>
        </Toolbar>
    ),
});

/** An app's menus, the ones that do not fit in one more. */
export const overflowMenubar = defineExample({
    of: Overflow,
    name: "menubar",
    description: "an app's menus, the ones that do not fit opening from one more",
    render: () => (
        <Overflow>
            <Menubar>
                <For each={["File", "Edit", "Selection", "View", "Go", "Run", "Terminal", "Help"]}>
                    {(menu) => (
                        <MenubarMenu>
                            <MenubarTrigger>{menu}</MenubarTrigger>
                            <MenubarContent>
                                <MenubarItem>{`${menu} one`}</MenubarItem>
                                <MenubarItem>{`${menu} two`}</MenubarItem>
                            </MenubarContent>
                        </MenubarMenu>
                    )}
                </For>
            </Menubar>
            <OverflowMenu />
        </Overflow>
    ),
});

/** Anything in a row, with a trigger and a menu of its own. */
export const overflowCustom = defineExample({
    of: Overflow,
    name: "custom",
    description: "badges in a row, with a trigger that counts and a menu of its own",
    properties: {},
    render: (properties) => <Labels {...properties} />,
});

/** Render the labels of an issue, counting those that do not fit. */
function Labels(properties: PropertiesOf<typeof Overflow>): JSX.Element {
    return (
        <Overflow {...properties}>
            <For each={["bug", "design", "accessibility", "timeline", "keyboard", "browser"]}>
                {(label) => (
                    <OverflowItem>
                        <Badge variant="secondary">{label}</Badge>
                    </OverflowItem>
                )}
            </For>
            <OverflowMenu>
                <OverflowMenuTrigger size="sm">More</OverflowMenuTrigger>
                <OverflowMenuContent>
                    {(entry) => <DropdownMenuItem>{entry.label()}</DropdownMenuItem>}
                </OverflowMenuContent>
            </OverflowMenu>
        </Overflow>
    );
}
