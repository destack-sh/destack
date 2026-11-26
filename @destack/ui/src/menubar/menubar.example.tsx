import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import {
    Menubar,
    MenubarCheckboxItem,
    MenubarContent,
    MenubarItem,
    MenubarMenu,
    MenubarRadioGroup,
    MenubarRadioItem,
    MenubarSeparator,
    MenubarShortcut,
    MenubarTrigger,
} from "./menubar.tsx";

/** An editor's File and Edit menus. */
export const menubarEditorMenus = defineExample({
    of: Menubar,
    name: "editorMenus",
    description: t`An editor's File and Edit menus.`,
    properties: {
        "aria-label": "Editor",
    },
    render: (properties) => (
        <Menubar {...properties}>
            <MenubarMenu>
                <MenubarTrigger>File</MenubarTrigger>
                <MenubarContent>
                    <MenubarItem>
                        New note
                        <MenubarShortcut keys="mod+n" />
                    </MenubarItem>
                    <MenubarSeparator />
                    <MenubarItem>Export</MenubarItem>
                </MenubarContent>
            </MenubarMenu>
            <MenubarMenu>
                <MenubarTrigger>Edit</MenubarTrigger>
                <MenubarContent>
                    <MenubarItem>
                        Undo
                        <MenubarShortcut keys="mod+z" />
                    </MenubarItem>
                </MenubarContent>
            </MenubarMenu>
        </Menubar>
    ),
});

/** The editor's File menu open below its trigger. */
export const menubarEditorMenusOpen = defineExample({
    of: Menubar,
    name: "editorMenusOpen",
    description: t`The editor's File menu open below its trigger.`,
    properties: {
        "aria-label": "Editor",
    },
    render: (properties) => (
        <Menubar {...properties}>
            <MenubarMenu defaultOpen>
                <MenubarTrigger>File</MenubarTrigger>
                <MenubarContent>
                    <MenubarItem>
                        New note
                        <MenubarShortcut keys="mod+n" />
                    </MenubarItem>
                    <MenubarSeparator />
                    <MenubarItem>Export</MenubarItem>
                    <MenubarItem disabled>Print</MenubarItem>
                </MenubarContent>
            </MenubarMenu>
            <MenubarMenu>
                <MenubarTrigger>Edit</MenubarTrigger>
                <MenubarContent>
                    <MenubarItem>Undo</MenubarItem>
                </MenubarContent>
            </MenubarMenu>
        </Menubar>
    ),
});

/** The editor's File menu open, laid out by its side and alignment. */
export const menubarContentDefault = defineExample({
    of: MenubarContent,
    name: "default",
    description: t`The editor's File menu open, laid out by its side and alignment.`,
    render: (properties) => (
        <Menubar aria-label="Editor">
            <MenubarMenu defaultOpen>
                <MenubarTrigger>File</MenubarTrigger>
                <MenubarContent {...properties}>
                    <MenubarItem>New note</MenubarItem>
                    <MenubarItem>Export</MenubarItem>
                </MenubarContent>
            </MenubarMenu>
        </Menubar>
    ),
});

/** The editor's File menu, exporting set on its own. */
export const menubarItemDefault = defineExample({
    of: MenubarItem,
    name: "default",
    description: t`The editor's File menu, exporting set on its own.`,
    render: (properties) => (
        <Menubar aria-label="Editor">
            <MenubarMenu defaultOpen>
                <MenubarTrigger>File</MenubarTrigger>
                <MenubarContent>
                    <MenubarItem>New note</MenubarItem>
                    <MenubarItem {...properties}>Export</MenubarItem>
                </MenubarContent>
            </MenubarMenu>
        </Menubar>
    ),
});

/** The editor's View menu, the ruler set on its own. */
export const menubarCheckboxItemDefault = defineExample({
    of: MenubarCheckboxItem,
    name: "default",
    description: t`The editor's View menu, the ruler set on its own.`,
    render: (properties) => (
        <Menubar aria-label="Editor">
            <MenubarMenu defaultOpen>
                <MenubarTrigger>View</MenubarTrigger>
                <MenubarContent>
                    <MenubarCheckboxItem checked {...properties}>
                        Ruler
                    </MenubarCheckboxItem>
                </MenubarContent>
            </MenubarMenu>
        </Menubar>
    ),
});

/** The editor's zoom levels, the largest set on its own. */
export const menubarRadioItemDefault = defineExample({
    of: MenubarRadioItem,
    name: "default",
    description: t`The editor's zoom levels, the largest set on its own.`,
    render: (properties) => (
        <Menubar aria-label="Editor">
            <MenubarMenu defaultOpen>
                <MenubarTrigger>View</MenubarTrigger>
                <MenubarContent>
                    <MenubarRadioGroup value="100">
                        <MenubarRadioItem value="100">100%</MenubarRadioItem>
                        <MenubarRadioItem value="200" {...properties}>
                            200%
                        </MenubarRadioItem>
                    </MenubarRadioGroup>
                </MenubarContent>
            </MenubarMenu>
        </Menubar>
    ),
});
