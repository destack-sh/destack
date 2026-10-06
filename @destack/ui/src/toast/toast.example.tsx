import { defineExample } from "@destack/package/declare";
import { Button } from "../button/index.ts";
import { Toaster, toast } from "./toast.tsx";

/** A toaster and a button that archives a note with an undo action. */
export const toasterArchiveUndo = defineExample({
    of: Toaster,
    name: "archive-undo",
    description: "a toaster and a button that archives a note with an undo action",
    render: () => (
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
    ),
});
