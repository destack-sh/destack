import { defineExample } from "@destack/package/declare";
import { createSignal, For, onSettled } from "@destack/view";
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
import { Toaster, toast } from "./toast.tsx";

/** The members of the workspace the members dialog lists. */
const MEMBERS = ["Ada Lovelace", "Jonas Weber", "Mia Sørensen"] as const;

/** A toaster and a button that archives a note with an undo action. */
export const toasterArchiveUndo = defineExample({
    of: Toaster,
    name: "archive-undo",
    description: "a toaster and a button that archives a note with an undo action",
    properties: {
        position: "bottom-right",
    },
    render: (properties) => (
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
            <Toaster {...properties} />
        </>
    ),
});

/** A toaster whose toasts take their kind's color, with buttons that save, fail and warn. */
export const toasterRichKinds = defineExample({
    of: Toaster,
    name: "rich-kinds",
    description:
        "a toaster whose toasts take their kind's color, with buttons that save, fail and warn",
    properties: {
        position: "top-center",
        richColors: true,
    },
    render: (properties) => (
        <>
            <Cluster>
                <Button onClick={() => toast.success("Note saved")}>Save</Button>
                <Button variant="outline" onClick={() => toast.error("Upload failed")}>
                    Upload
                </Button>
                <Button variant="outline" onClick={() => toast.warning("Storage almost full")}>
                    Check storage
                </Button>
            </Cluster>
            <Toaster {...properties} />
        </>
    ),
});

/** A lost connection's toast that stays while it retries, then turns into one that it is back. */
export const toasterReconnecting = defineExample({
    of: Toaster,
    name: "reconnecting",
    description:
        "a lost connection's toast that stays while it retries, then turns into one that it is back",
    properties: {
        position: "bottom-right",
    },
    render: (properties) => (
        <>
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
            <Toaster {...properties} />
        </>
    ),
});

/** A toast shown with its undo action until someone dismisses it. */
export const toasterArchiveUndoOpen = defineExample({
    of: Toaster,
    name: "archive-undo-open",
    description: "a toast shown with its undo action until someone dismisses it",
    properties: {
        position: "bottom-right",
    },
    render: (properties) => {
        // show the toast once the toaster is on the page, dismissing it when the example goes
        onSettled(() => {
            const id = toast("Note archived", {
                duration: Number.POSITIVE_INFINITY,
                action: { label: "Undo", onClick: () => toast("Note restored") },
            });

            return () => toast.dismiss(id);
        });

        return <Toaster {...properties} />;
    },
});

/** A members dialog whose removals toast an undo that works while the dialog stays open. */
export const toasterMembersDialog = defineExample({
    of: Toaster,
    name: "members-dialog",
    description: "a members dialog whose removals toast an undo that works while it stays open",
    properties: {
        position: "bottom-right",
    },
    render: (properties) => {
        // remove a member, and put them back in place on undo
        const [members, setMembers] = createSignal<readonly string[]>(MEMBERS);
        const remove = (name: string) => {
            setMembers((current) => current.filter((member) => member !== name));
            toast(`Removed ${name}`, {
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
            <>
                <Dialog>
                    <DialogTrigger
                        render={(attributes) => <Button {...attributes} variant="outline" />}
                    >
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
                                        <Button
                                            variant="ghost"
                                            size="sm"
                                            onClick={() => remove(name)}
                                        >
                                            Remove
                                        </Button>
                                    </Cluster>
                                )}
                            </For>
                        </Stack>
                    </DialogContent>
                </Dialog>
                <Toaster {...properties} />
            </>
        );
    },
});
