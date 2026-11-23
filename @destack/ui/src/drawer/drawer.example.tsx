import { defineExample } from "@destack/package/declare";
import { Button } from "../button/index.ts";
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
            <DrawerTrigger render={(attributes) => <Button {...attributes} variant="outline" />}>
                Share
            </DrawerTrigger>
            <DrawerContent>
                <DrawerHeader>
                    <DrawerTitle>Share Groceries</DrawerTitle>
                    <DrawerDescription>People with the link can view the note.</DrawerDescription>
                </DrawerHeader>
                <DrawerFooter>
                    <DrawerClose
                        render={(attributes) => <Button {...attributes} variant="outline" />}
                    >
                        Cancel
                    </DrawerClose>
                    <DrawerClose render={(attributes) => <Button {...attributes} />}>
                        Copy link
                    </DrawerClose>
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
    properties: {
        defaultOpen: true,
    },
    render: (properties) => (
        <Drawer {...properties}>
            <DrawerTrigger render={(attributes) => <Button {...attributes} variant="outline" />}>
                Share
            </DrawerTrigger>
            <DrawerContent>
                <DrawerHeader>
                    <DrawerTitle>Share Groceries</DrawerTitle>
                    <DrawerDescription>People with the link can view the note.</DrawerDescription>
                </DrawerHeader>
                <DrawerFooter>
                    <DrawerClose
                        render={(attributes) => <Button {...attributes} variant="outline" />}
                    >
                        Cancel
                    </DrawerClose>
                    <DrawerClose render={(attributes) => <Button {...attributes} />}>
                        Copy link
                    </DrawerClose>
                </DrawerFooter>
            </DrawerContent>
        </Drawer>
    ),
});

/** The comments of a note in a drawer that rests at half the screen or all of it. */
export const drawerNoteComments = defineExample({
    of: Drawer,
    name: "note-comments",
    description: "the comments of a note in a drawer that rests at half the screen or all of it",
    render: () => (
        <Drawer defaultOpen snapPoints={[0.5, 1]} defaultActiveSnapPoint={0.5}>
            <DrawerTrigger render={(attributes) => <Button {...attributes} variant="outline" />}>
                Comments
            </DrawerTrigger>
            <DrawerContent>
                <DrawerHeader>
                    <DrawerTitle>Comments</DrawerTitle>
                    <DrawerDescription>Drag up to read every comment.</DrawerDescription>
                </DrawerHeader>
            </DrawerContent>
        </Drawer>
    ),
});

/** A drawer that shares a note, and opens another one over it to pick a person. */
export const drawerPickPerson = defineExample({
    of: Drawer,
    name: "pick-person",
    description: "a drawer that shares a note, and opens another one over it to pick a person",
    render: () => (
        <Drawer defaultOpen>
            <DrawerContent>
                <DrawerHeader>
                    <DrawerTitle>Share Groceries</DrawerTitle>
                    <DrawerDescription>Choose who can see the note.</DrawerDescription>
                </DrawerHeader>
                <DrawerFooter>
                    <Drawer>
                        <DrawerTrigger
                            render={(attributes) => <Button {...attributes} variant="outline" />}
                        >
                            Add person
                        </DrawerTrigger>
                        <DrawerContent>
                            <DrawerHeader>
                                <DrawerTitle>Add person</DrawerTitle>
                                <DrawerDescription>They can read and change it.</DrawerDescription>
                            </DrawerHeader>
                            <DrawerFooter>
                                <DrawerClose render={(attributes) => <Button {...attributes} />}>
                                    Add Grace
                                </DrawerClose>
                            </DrawerFooter>
                        </DrawerContent>
                    </Drawer>
                </DrawerFooter>
            </DrawerContent>
        </Drawer>
    ),
});
