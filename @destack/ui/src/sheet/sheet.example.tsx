import { defineExample } from "@destack/package/declare";
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

/** The filters of a note list in a panel from the right. */
export const sheetListFilters = defineExample({
    of: Sheet,
    name: "list-filters",
    description: "the filters of a note list in a panel from the right",
    render: () => (
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
    ),
});

/** The filters panel of a note list open from the right. */
export const sheetListFiltersOpen = defineExample({
    of: Sheet,
    name: "list-filters-open",
    description: "the filters panel of a note list open from the right",
    render: () => (
        <Sheet defaultOpen>
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
    ),
});
