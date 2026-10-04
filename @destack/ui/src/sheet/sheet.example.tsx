import type { JSX } from "@solidjs/web";
import { Checkbox } from "../checkbox/index.ts";
import { Label } from "../label/index.ts";
import {
    Sheet,
    SheetClose,
    SheetContent,
    SheetDescription,
    SheetFooter,
    SheetHeader,
    SheetTitle,
    SheetTrigger,
} from "./sheet.tsx";

/** Show the filters of a note list in a panel from the right. */
export function SheetExample(): JSX.Element {
    return (
        <Sheet>
            <SheetTrigger variant="outline">Filters</SheetTrigger>
            <SheetContent side="right">
                <SheetHeader>
                    <SheetTitle>Filters</SheetTitle>
                    <SheetDescription>Show only the notes that match.</SheetDescription>
                </SheetHeader>
                <Label>
                    <Checkbox name="shared" /> Shared with me
                </Label>
                <SheetFooter>
                    <SheetClose>Apply</SheetClose>
                </SheetFooter>
            </SheetContent>
        </Sheet>
    );
}
