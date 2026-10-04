import type { JSX } from "@solidjs/web";
import { createSignal, Show } from "solid-js";
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

/** Show a confirmation before a notebook and its notes are deleted. */
export function AlertDialogExample(): JSX.Element {
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
                        <AlertDialogAction variant="destructive" onClick={() => setDeleted(true)}>
                            Delete
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </Show>
    );
}
