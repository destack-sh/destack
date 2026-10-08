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
    name: "editor-menus",
    description: "an editor's File and Edit menus",
    render: () => (
        <Menubar aria-label="Editor">
            <MenubarMenu>
                <MenubarTrigger>File</MenubarTrigger>
                <MenubarContent>
                    <MenubarItem>
                        New note
                        <MenubarShortcut>⌘N</MenubarShortcut>
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
                        <MenubarShortcut>⌘Z</MenubarShortcut>
                    </MenubarItem>
                </MenubarContent>
            </MenubarMenu>
        </Menubar>
    ),
});

/** The editor's File menu open below its trigger. */
export const menubarEditorMenusOpen = defineExample({
    of: Menubar,
    name: "editor-menus-open",
    description: "the editor's File menu open below its trigger",
    render: () => (
        <Menubar aria-label="Editor">
            <MenubarMenu defaultOpen>
                <MenubarTrigger>File</MenubarTrigger>
                <MenubarContent>
                    <MenubarItem>
                        New note
                        <MenubarShortcut>⌘N</MenubarShortcut>
                    </MenubarItem>
                    <MenubarSeparator />
                    <MenubarItem>Export</MenubarItem>
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
