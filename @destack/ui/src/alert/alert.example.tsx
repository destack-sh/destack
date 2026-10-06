import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/icon";
import { Alert, AlertDescription, AlertTitle } from "./alert.tsx";

/** A failed sync with what to do about it. */
export const alertFailedSync = defineExample({
    of: Alert,
    name: "failed-sync",
    description: "a failed sync with what to do about it",
    render: () => (
        <Alert variant="destructive">
            <Icon name="warning-circle" />
            <AlertTitle>Sync paused</AlertTitle>
            <AlertDescription>
                Your notebooks are over the storage limit. Free up space or upgrade to keep syncing.
            </AlertDescription>
        </Alert>
    ),
});
