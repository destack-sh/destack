import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import {
    Menubar,
    MenubarContent,
    MenubarItem,
    MenubarMenu,
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
