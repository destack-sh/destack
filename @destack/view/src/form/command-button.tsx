import { t } from "@destack/locale";
import type { Submission } from "@destack/object/client";
import { Button, type ButtonProperties } from "@destack/ui/button";
import { Spinner } from "@destack/ui/spinner";
import { toast } from "@destack/ui/toast";
import { useLocale } from "@destack/locale/solid";
import type { JSX } from "../solid/component.ts";
import { Show } from "../solid/flow.ts";
import { createSignal, omit } from "../solid/reactive.ts";

/** Where a command button's last run stands: none yet, waiting for the server, or refused. */
export type CommandButtonStatus = "idle" | "pending" | "failed";

/** The properties of a command button, a button's properties included. */
export interface CommandButtonProperties extends Omit<ButtonProperties, "onClick"> {
    /** Run the object method, such as `space.mutate.note.archive({ id })`, returning its submission or promise. */
    readonly run: () => Submission<unknown> | Promise<unknown>;
    /** The toast text when the run fails, a generic one by default. */
    readonly errorMessage?: string;
}

/** Render a button that runs an object method, busy until the server confirms it and toasting a refusal. */
export function CommandButton(properties: CommandButtonProperties): JSX.Element {
    // follow the last run
    const locale = useLocale();
    const [status, setStatus] = createSignal<CommandButtonStatus>("idle");
    const rest = omit(properties, "run", "errorMessage", "children");

    // run once at a time and toast a refusal
    const run = () => {
        if (status() === "pending") {
            return;
        }
        setStatus("pending");
        const running = properties.run();
        const settled = running instanceof Promise ? running : running.confirmed;
        settled.then(
            () => setStatus("idle"),
            (error: unknown) => {
                setStatus("failed");
                const message = properties.errorMessage ?? locale.render(t`That did not work`);
                toast.error(message, error instanceof Error ? { description: error.message } : {});
            },
        );
    };

    return (
        <Button
            data-slot="command-button"
            data-state={status()}
            aria-busy={status() === "pending" ? "true" : undefined}
            aria-disabled={status() === "pending" ? "true" : undefined}
            {...rest}
            onClick={run}
        >
            <Show when={status() === "pending"}>
                <Spinner />
            </Show>
            {properties.children}
        </Button>
    );
}
