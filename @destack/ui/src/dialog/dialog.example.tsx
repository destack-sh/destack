import { defineExample } from "@destack/package/declare";
import { Field, FieldLabel } from "../field/index.ts";
import { Input } from "../input/index.ts";
import {
    Dialog,
    DialogClose,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "./dialog.tsx";

/** A dialog that renames a note from a form. */
export const dialogRenameNote = defineExample({
    of: Dialog,
    name: "rename-note",
    description: "a dialog that renames a note from a form",
    render: () => (
        <Dialog>
            <DialogTrigger variant="outline">Rename</DialogTrigger>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>Rename note</DialogTitle>
                    <DialogDescription>
                        The new title shows everywhere the note is shared.
                    </DialogDescription>
                </DialogHeader>
                <Field>
                    <FieldLabel>Title</FieldLabel>
                    <Input name="title" value="Groceries" />
                </Field>
                <DialogFooter>
                    <DialogClose variant="outline">Cancel</DialogClose>
                    <DialogClose type="submit">Save</DialogClose>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    ),
});

/** The rename dialog open over the page. */
export const dialogRenameNoteOpen = defineExample({
    of: Dialog,
    name: "rename-note-open",
    description: "the rename dialog open over the page",
    render: () => (
        <Dialog defaultOpen>
            <DialogTrigger variant="outline">Rename</DialogTrigger>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>Rename note</DialogTitle>
                    <DialogDescription>
                        The new title shows everywhere the note is shared.
                    </DialogDescription>
                </DialogHeader>
                <Field>
                    <FieldLabel>Title</FieldLabel>
                    <Input name="title" value="Groceries" />
                </Field>
                <DialogFooter>
                    <DialogClose variant="outline">Cancel</DialogClose>
                    <DialogClose type="submit">Save</DialogClose>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    ),
});
