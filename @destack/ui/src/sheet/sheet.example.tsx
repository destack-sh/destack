import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Button } from "../button/index.ts";
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
    name: "listFilters",
    description: t`The filters of a note list in a panel from the right.`,
    render: () => (
        <Sheet>
            <SheetTrigger render={(attributes) => <Button {...attributes} variant="outline" />}>
                Filters
            </SheetTrigger>
            <SheetContent side="right">
                <SheetHeader>
                    <SheetTitle>Filters</SheetTitle>
                    <SheetDescription>Show only the notes that match.</SheetDescription>
                </SheetHeader>
                <Label>
                    <Checkbox name="shared" /> Shared with me
                </Label>
                <SheetFooter>
                    <SheetClose render={(attributes) => <Button {...attributes} />}>
                        Apply
                    </SheetClose>
                </SheetFooter>
            </SheetContent>
        </Sheet>
    ),
});

/** The filters panel of a note list open from the right. */
export const sheetListFiltersOpen = defineExample({
    of: Sheet,
    name: "listFiltersOpen",
    description: t`The filters panel of a note list open from the right.`,
    properties: {
        defaultOpen: true,
    },
    render: (properties) => (
        <Sheet {...properties}>
            <SheetTrigger render={(attributes) => <Button {...attributes} variant="outline" />}>
                Filters
            </SheetTrigger>
            <SheetContent side="right">
                <SheetHeader>
                    <SheetTitle>Filters</SheetTitle>
                    <SheetDescription>Show only the notes that match.</SheetDescription>
                </SheetHeader>
                <Label>
                    <Checkbox name="shared" /> Shared with me
                </Label>
                <SheetFooter>
                    <SheetClose render={(attributes) => <Button {...attributes} />}>
                        Apply
                    </SheetClose>
                </SheetFooter>
            </SheetContent>
        </Sheet>
    ),
});
