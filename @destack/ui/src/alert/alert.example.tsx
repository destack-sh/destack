import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/ui/icon";
import { Alert, AlertDescription, AlertTitle } from "./alert.tsx";

/** A failed sync with what to do about it. */
export const alertFailedSync = defineExample({
    of: Alert,
    name: "failed-sync",
    description: "a failed sync with what to do about it",
    properties: {
        variant: "destructive",
    },
    render: (properties) => (
        <Alert {...properties}>
            <Icon name="warning-circle" />
            <AlertTitle>Sync paused</AlertTitle>
            <AlertDescription>
                Your notebooks are over the storage limit. Free up space or upgrade to keep syncing.
            </AlertDescription>
        </Alert>
    ),
});

/** The same notice at its default look. */
export const alertSyncNotice = defineExample({
    of: Alert,
    name: "sync-notice",
    description: "the same notice at its default look",
    properties: {
        variant: "default",
    },
    render: (properties) => (
        <Alert {...properties}>
            <Icon name="warning-circle" />
            <AlertTitle>Sync paused</AlertTitle>
            <AlertDescription>
                Your notebooks are over the storage limit. Free up space or upgrade to keep syncing.
            </AlertDescription>
        </Alert>
    ),
});
