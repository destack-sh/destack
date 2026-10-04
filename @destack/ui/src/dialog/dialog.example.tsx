import type { JSX } from "@solidjs/web";
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

/** Show a dialog that renames a note from a form. */
export function DialogExample(): JSX.Element {
    return (
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
    );
}
