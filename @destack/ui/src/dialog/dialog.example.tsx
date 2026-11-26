import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Button } from "../button/index.ts";
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
export const dialogDefault = defineExample({
    of: Dialog,
    name: "default",
    description: t`A dialog that renames a note from a form.`,
    render: () => (
        <Dialog>
            <DialogTrigger render={(attributes) => <Button {...attributes} variant="outline" />}>
                Rename
            </DialogTrigger>
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
                    <DialogClose
                        render={(attributes) => <Button {...attributes} variant="outline" />}
                    >
                        Cancel
                    </DialogClose>
                    <DialogClose render={(attributes) => <Button {...attributes} type="submit" />}>
                        Save
                    </DialogClose>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    ),
});

/** The rename dialog open over the page. */
export const dialogOpen = defineExample({
    of: Dialog,
    name: "open",
    description: t`The rename dialog open over the page.`,
    properties: {
        defaultOpen: true,
    },
    render: (properties) => (
        <Dialog {...properties}>
            <DialogTrigger render={(attributes) => <Button {...attributes} variant="outline" />}>
                Rename
            </DialogTrigger>
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
                    <DialogClose
                        render={(attributes) => <Button {...attributes} variant="outline" />}
                    >
                        Cancel
                    </DialogClose>
                    <DialogClose render={(attributes) => <Button {...attributes} type="submit" />}>
                        Save
                    </DialogClose>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    ),
});
