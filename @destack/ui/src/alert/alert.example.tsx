import { t } from "@destack/locale";
import warningCircle from "@destack/icon/phosphor/warning-circle";
import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/ui/icon";
import { createSignal, Show } from "@destack/view";
import { Button } from "../button/index.ts";
import { Alert, AlertAction, AlertClose, AlertDescription, AlertTitle } from "./alert.tsx";

/** A failed sync with what to do about it. */
export const alertFailedSync = defineExample({
    of: Alert,
    name: "failedSync",
    description: t`A failed sync with what to do about it.`,
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
    name: "syncNotice",
    description: t`The same notice at its default look.`,
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

/** A failed call with a retry, which a person may dismiss. */
export const alertRetry = defineExample({
    of: Alert,
    name: "retry",
    description: t`A failed call with a retry, which a person may dismiss.`,
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
