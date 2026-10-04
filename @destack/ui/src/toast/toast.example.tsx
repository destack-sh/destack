import type { JSX } from "@solidjs/web";
import { Button } from "../button/index.ts";
import { Toaster, toast } from "./toast.tsx";

/** Show a toaster and a button that archives a note with an undo action. */
export function ToastExample(): JSX.Element {
    return (
        <>
            <Button
                onClick={() =>
                    toast("Note archived", {
                        action: { label: "Undo", onClick: () => toast("Note restored") },
                    })
                }
            >
                Archive
            </Button>
            <Toaster position="bottom-right" />
        </>
    );
}
