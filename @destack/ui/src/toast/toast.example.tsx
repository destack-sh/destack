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

/** A toaster whose toasts take their kind's color, with buttons that save, fail and warn. */
export const toasterRichKinds = defineExample({
    of: Toaster,
    name: "rich-kinds",
    description:
        "a toaster whose toasts take their kind's color, with buttons that save, fail and warn",
    render: () => (
        <>
            <Button onClick={() => toast.success("Note saved")}>Save</Button>
            <Button onClick={() => toast.error("Upload failed")}>Upload</Button>
            <Button onClick={() => toast.warning("Storage almost full")}>Check storage</Button>
            <Toaster position="top-center" richColors />
        </>
    ),
});
