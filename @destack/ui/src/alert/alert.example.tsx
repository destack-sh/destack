import { Icon } from "@destack/icon";
import type { JSX } from "@solidjs/web";
import { Alert, AlertDescription, AlertTitle } from "./alert.tsx";

/** Show a failed sync with what to do about it. */
export function AlertExample(): JSX.Element {
    return (
        <Alert variant="destructive">
            <Icon name="warning-circle" />
            <AlertTitle>Sync paused</AlertTitle>
            <AlertDescription>
                Your notebooks are over the storage limit. Free up space or upgrade to keep syncing.
            </AlertDescription>
        </Alert>
    );
}
