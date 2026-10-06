import { defineExample } from "@destack/package/declare";
import { createSignal, Show } from "@destack/view";
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
    AlertDialogTrigger,
} from "./alert-dialog.tsx";

/** A confirmation before a notebook and its notes are deleted. */
export const alertDialogDeleteNotebook = defineExample({
    of: AlertDialog,
    name: "delete-notebook",
    description: "a confirmation before a notebook and its notes are deleted",
    render: () => {
        const [isDeleted, setDeleted] = createSignal(false);

        return (
            <Show when={!isDeleted()} fallback={<p role="status">Trips deleted</p>}>
                <AlertDialog>
                    <AlertDialogTrigger variant="destructive">Delete notebook</AlertDialogTrigger>
                    <AlertDialogContent>
                        <AlertDialogHeader>
                            <AlertDialogTitle>Delete Trips?</AlertDialogTitle>
                            <AlertDialogDescription>
                                The notebook and its 12 notes are deleted for everyone.
                            </AlertDialogDescription>
                        </AlertDialogHeader>
                        <AlertDialogFooter>
                            <AlertDialogCancel>Cancel</AlertDialogCancel>
                            <AlertDialogAction
                                variant="destructive"
                                onClick={() => setDeleted(true)}
                            >
                                Delete
                            </AlertDialogAction>
                        </AlertDialogFooter>
                    </AlertDialogContent>
                </AlertDialog>
            </Show>
        );
    },
});

/** The confirmation to delete a notebook open over the page. */
export const alertDialogDeleteNotebookOpen = defineExample({
    of: AlertDialog,
    name: "delete-notebook-open",
    description: "the confirmation to delete a notebook open over the page",
    render: () => (
        <AlertDialog defaultOpen>
            <AlertDialogTrigger variant="destructive">Delete notebook</AlertDialogTrigger>
            <AlertDialogContent>
                <AlertDialogHeader>
                    <AlertDialogTitle>Delete Trips?</AlertDialogTitle>
                    <AlertDialogDescription>
                        The notebook and its 12 notes are deleted for everyone.
                    </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogCancel>Cancel</AlertDialogCancel>
                    <AlertDialogAction variant="destructive">Delete</AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    ),
});
