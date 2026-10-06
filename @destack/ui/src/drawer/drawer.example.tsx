import { defineExample } from "@destack/package/declare";
import {
    Drawer,
    DrawerClose,
    DrawerContent,
    DrawerDescription,
    DrawerFooter,
    DrawerHeader,
    DrawerTitle,
    DrawerTrigger,
} from "./drawer.tsx";

/** The sharing options of a note in a drawer from the bottom. */
export const drawerShareNote = defineExample({
    of: Drawer,
    name: "share-note",
    description: "the sharing options of a note in a drawer from the bottom",
    render: () => (
        <Drawer>
            <DrawerTrigger variant="outline">Share</DrawerTrigger>
            <DrawerContent>
                <DrawerHeader>
                    <DrawerTitle>Share Groceries</DrawerTitle>
                    <DrawerDescription>People with the link can view the note.</DrawerDescription>
                </DrawerHeader>
                <DrawerFooter>
                    <DrawerClose>Copy link</DrawerClose>
                    <DrawerClose variant="outline">Cancel</DrawerClose>
                </DrawerFooter>
            </DrawerContent>
        </Drawer>
    ),
});

/** The sharing drawer of a note open from the bottom. */
export const drawerShareNoteOpen = defineExample({
    of: Drawer,
    name: "share-note-open",
    description: "the sharing drawer of a note open from the bottom",
    render: () => (
        <Drawer defaultOpen>
            <DrawerTrigger variant="outline">Share</DrawerTrigger>
            <DrawerContent>
                <DrawerHeader>
                    <DrawerTitle>Share Groceries</DrawerTitle>
                    <DrawerDescription>People with the link can view the note.</DrawerDescription>
                </DrawerHeader>
                <DrawerFooter>
                    <DrawerClose>Copy link</DrawerClose>
                    <DrawerClose variant="outline">Cancel</DrawerClose>
                </DrawerFooter>
            </DrawerContent>
        </Drawer>
    ),
});
