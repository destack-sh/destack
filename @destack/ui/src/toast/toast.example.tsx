import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { createSignal, For, type JSX, onSettled } from "@destack/view";
import { Button } from "../button/index.ts";
import { Cluster } from "../cluster/index.ts";
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "../dialog/index.ts";
import { Stack } from "../stack/index.ts";
import { Toaster, useToast } from "./toast.tsx";

/** The members of the workspace the members dialog lists. */
const MEMBERS = ["Ada Lovelace", "Jonas Weber", "Mia Sørensen"] as const;

/** A button that archives a note with an undo action. */
export const toasterDefault = defineExample({
    of: Toaster,
    name: "default",
    description: t`A button that archives a note with an undo action.`,
    properties: {
        position: "bottom-right",
    },
    render: (properties) => (
        <Toaster {...properties}>
            <Archive />
        </Toaster>
    ),
});

/** Buttons that save, fail and warn, a toast of each kind. */
export const toasterKinds = defineExample({
    of: Toaster,
    name: "kinds",
    description: t`Buttons that save, fail and warn, a toast of each kind.`,
    render: (properties) => (
        <Toaster {...properties}>
            <Kinds />
        </Toaster>
    ),
});

/** A lost connection's toast that stays while it retries, then turns into one that it is back. */
export const toasterLoading = defineExample({
    of: Toaster,
    name: "loading",
    description: t`A lost connection's toast that stays while it retries, then turns into one that it is back.`,
    render: (properties) => (
        <Toaster {...properties}>
            <Connection />
        </Toaster>
    ),
});

/** A toast shown with its undo action until someone dismisses it. */
export const toasterOpen = defineExample({
    of: Toaster,
    name: "open",
    description: t`A toast shown with its undo action until someone dismisses it.`,
    render: (properties) => (
        <Toaster {...properties}>
            <Archived />
        </Toaster>
    ),
});

/** Three toasts in a stack, with a long error in front. */
export const toasterStacked = defineExample({
    of: Toaster,
    name: "stacked",
    description: t`Three toasts in a stack, with a long error in front.`,
    render: (properties) => (
        <Toaster {...properties}>
            <Stacked />
        </Toaster>
    ),
});

/** A members dialog whose removals toast an undo that works while the dialog stays open. */
export const toasterInDialog = defineExample({
    of: Toaster,
    name: "inDialog",
    description: t`A members dialog whose removals toast an undo that works while it stays open.`,
    render: (properties) => (
        <Toaster {...properties}>
            <Members />
        </Toaster>
    ),
});

/** Render a button that archives a note. */
function Archive(): JSX.Element {
    const toast = useToast();

    return (
        <Button
            onClick={() =>
                toast.show("Note archived", {
                    action: { label: "Undo", onClick: () => toast.show("Note restored") },
                })
            }
        >
            Archive
        </Button>
    );
}

/** Render a button per kind of toast. */
function Kinds(): JSX.Element {
    const toast = useToast();

    return (
        <Cluster>
            <Button onClick={() => toast.success("Note saved")}>Save</Button>
            <Button variant="outline" onClick={() => toast.error("Upload failed")}>
                Upload
            </Button>
            <Button variant="outline" onClick={() => toast.warning("Storage almost full")}>
                Check storage
            </Button>
        </Cluster>
    );
}

/** Render buttons that lose a connection and bring it back. */
function Connection(): JSX.Element {
    const toast = useToast();

    return (
        <Cluster>
            <Button
                variant="outline"
                onClick={() =>
                    toast.loading("Connection lost", {
                        id: "connection",
                        description: "Retrying…",
                    })
                }
            >
                Lose connection
            </Button>
            <Button
                variant="outline"
                onClick={() => toast.success("Back online", { id: "connection" })}
            >
                Reconnect
            </Button>
        </Cluster>
    );
}

/** Show a toast of an archived note once mounted. */
function Archived(): JSX.Element {
    const toast = useToast();
    onSettled(() => {
        toast.show("Note archived", {
            duration: Number.POSITIVE_INFINITY,
            action: { label: "Undo", onClick: () => toast.show("Note restored") },
        });
    });

    return null;
}

/** Show three toasts once mounted, the last one long. */
function Stacked(): JSX.Element {
    const toast = useToast();
    onSettled(() => {
        toast.success("Note saved", { duration: Number.POSITIVE_INFINITY });
        toast.show("Moved to Trips", {
            duration: Number.POSITIVE_INFINITY,
            action: { label: "Undo", onClick: () => toast.show("Moved back") },
        });
        toast.error("Upload failed", {
            description:
                "The file quarterly-report-final-version-2026.pdf is 214 MB. A file may have up to 100 MB in this space.",
            action: { label: "Try again", onClick: () => toast.show("Uploading") },
        });
    });

    return null;
}

/** Render a members dialog whose removals toast an undo. */
function Members(): JSX.Element {
    // remove a member, and put them back in place on undo
    const toast = useToast();
    const [members, setMembers] = createSignal<readonly string[]>(MEMBERS);
    const remove = (name: string) => {
        setMembers((current) => current.filter((member) => member !== name));
        toast.show(`Removed ${name}`, {
            action: {
                label: "Undo",
                onClick: () =>
                    setMembers((current) =>
                        MEMBERS.filter((member) => member === name || current.includes(member)),
                    ),
            },
        });
    };

    return (
        <Dialog>
            <DialogTrigger render={(attributes) => <Button {...attributes} variant="outline" />}>
                Members
            </DialogTrigger>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>Members</DialogTitle>
                </DialogHeader>
                <Stack>
                    <For each={members()}>
                        {(name) => (
                            <Cluster data-slot="member">
                                {name}
                                <Button variant="ghost" size="sm" onClick={() => remove(name)}>
                                    Remove
                                </Button>
                            </Cluster>
                        )}
                    </For>
                </Stack>
            </DialogContent>
        </Dialog>
    );
}
