import type { JSX } from "@solidjs/web";
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

/** Show the sharing options of a note in a drawer from the bottom. */
export function DrawerExample(): JSX.Element {
    return (
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
    );
}
