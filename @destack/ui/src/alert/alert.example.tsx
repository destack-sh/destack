import bell from "@destack/icon/phosphor/bell";
import checkCircle from "@destack/icon/phosphor/check-circle";
import info from "@destack/icon/phosphor/info";
import warning from "@destack/icon/phosphor/warning";
import warningCircle from "@destack/icon/phosphor/warning-circle";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { Icon } from "@destack/ui/icon";
import { createSignal, Show } from "@destack/view";
import { Button } from "../button/index.ts";
import { Stack } from "../stack/index.ts";
import { Alert, AlertAction, AlertClose, AlertDescription, AlertTitle } from "./alert.tsx";

/** The styles of a list of statuses. */
const styles = style.create({
    statuses: { maxWidth: "100%" },
});

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
            <Icon icon={warningCircle} />
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
            <Icon icon={warningCircle} />
            <AlertTitle>Sync paused</AlertTitle>
            <AlertDescription>
                Your notebooks are over the storage limit. Free up space or upgrade to keep syncing.
            </AlertDescription>
        </Alert>
    ),
});

/** Each status an alert shows, from a plain notice to a failure. */
export const alertStatuses = defineExample({
    of: Alert,
    name: "statuses",
    description: "each status an alert shows, from a plain notice to a failure",
    render: () => (
        <Stack xstyle={styles.statuses}>
            <Alert>
                <Icon icon={bell} />
                <AlertTitle>Sync runs every few minutes</AlertTitle>
            </Alert>
            <Alert variant="info">
                <Icon icon={info} />
                <AlertTitle>A new version is ready</AlertTitle>
                <AlertDescription>It loads the next time you open the app.</AlertDescription>
            </Alert>
            <Alert variant="success">
                <Icon icon={checkCircle} />
                <AlertTitle>Import finished</AlertTitle>
                <AlertDescription>All 248 notes are in your notebook.</AlertDescription>
            </Alert>
            <Alert variant="warning">
                <Icon icon={warning} />
                <AlertTitle>Storage almost full</AlertTitle>
                <AlertDescription>Notebooks stop syncing at 100%.</AlertDescription>
            </Alert>
            <Alert variant="destructive">
                <Icon icon={warningCircle} />
                <AlertTitle>Sync paused</AlertTitle>
                <AlertDescription>Your notebooks are over the storage limit.</AlertDescription>
            </Alert>
        </Stack>
    ),
});

/** A failed call with a retry, which a person may dismiss. */
export const alertRetry = defineExample({
    of: Alert,
    name: "retry",
    description: "a failed call with a retry, which a person may dismiss",
    properties: {
        variant: "destructive",
    },
    render: (properties) => {
        const [isShown, setShown] = createSignal(true);

        return (
            <Show when={isShown()}>
                <Alert {...properties}>
                    <Icon icon={warningCircle} />
                    <AlertTitle>Couldn't save the note</AlertTitle>
                    <AlertDescription>
                        The server refused the change. Your text is safe.
                    </AlertDescription>
                    <AlertAction>
                        <Button size="sm" variant="outline">
                            Retry
                        </Button>
                    </AlertAction>
                    <AlertClose onClick={() => setShown(false)} />
                </Alert>
            </Show>
        );
    },
});
