import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Refusal } from "./refusal.tsx";

/** The app a request asked for, as the gateway names it. */
const TASKS = { name: "tasks", title: "Tasks" };

/** The space the app is in. */
const STUDIO = { handle: "ada", title: "Studio" };

/** Where a person goes home to. */
const HOME = "https://home--studio--ada.destack.localhost/";

/** An app starting, which the page tries again on its own. */
export const refusalStarting = defineExample({
    of: Refusal,
    name: "starting",
    description: t`Tasks starting, the page trying again on its own.`,
    layout: "fullscreen",
    render: () => (
        <Refusal
            kind="starting"
            package={TASKS}
            space={STUDIO}
            action={{ kind: "retry", after: 60_000 }}
        />
    ),
});

/** An app that failed to start, with what failed under its details. */
export const refusalFailed = defineExample({
    of: Refusal,
    name: "failed",
    description: t`Tasks failing to start, with its error under the details.`,
    layout: "fullscreen",
    render: () => (
        <Refusal
            kind="failed"
            package={TASKS}
            space={STUDIO}
            error={{
                reason: "INSTANCE_FAILED",
                message: "the view's module threw: list is not defined",
            }}
            action={{ kind: "home", url: HOME }}
        />
    ),
});

/** An address with nothing at it. */
export const refusalNotFound = defineExample({
    of: Refusal,
    name: "notFound",
    description: t`An address with nothing at it, and the way home.`,
    layout: "fullscreen",
    render: () => <Refusal kind="not-found" action={{ kind: "home", url: HOME }} />,
});

/** An app the person has no access to. */
export const refusalForbidden = defineExample({
    of: Refusal,
    name: "forbidden",
    description: t`Tasks the person has no access to, and the way home.`,
    layout: "fullscreen",
    render: () => (
        <Refusal
            kind="forbidden"
            package={TASKS}
            space={STUDIO}
            action={{ kind: "home", url: HOME }}
        />
    ),
});

/** An app that needs the person signed in. */
export const refusalSignedOut = defineExample({
    of: Refusal,
    name: "signedOut",
    description: t`Tasks asking the person to sign in.`,
    layout: "fullscreen",
    render: () => (
        <Refusal
            kind="signed-out"
            package={TASKS}
            action={{ kind: "sign-in", url: "https://destack.localhost/sign-in" }}
        />
    ),
});

/** A device without a network. */
export const refusalOffline = defineExample({
    of: Refusal,
    name: "offline",
    description: t`A device without a network, offering to try again.`,
    layout: "fullscreen",
    render: () => <Refusal kind="offline" action={{ kind: "retry", after: 0 }} />,
});
