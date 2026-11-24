import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, swatch } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    createContext,
    createEffect,
    flush,
    type FormControl,
    type JSX,
    omit,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from "../alert-dialog/index.ts";
import { Button, type ButtonProperties } from "../button/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** What a form asks of its draft. */
type Draft = Pick<FormControl<never>, "status" | "error" | "isEdited" | "submit" | "reset">;

/** The draft of the nearest form, null outside one. */
const FormContext = createContext<Draft | null>(null);

/** The styles of a form's parts. */
const styles = style.create({
    error: {
        color: swatch.destructiveText,
    },
    status: {
        color: color.mutedForeground,
    },
});

/** The properties of a form, the native element's attributes included. */
export type FormProperties = Omit<JSX.FormHTMLAttributes<HTMLFormElement>, "class"> &
    ElementPartProperties & {
        /** What it edits, from `useForm`. */
        readonly form: Draft;
    };

/** Render a form that submits its draft, focusing the first control that is refused. */
export function Form(properties: FormProperties): JSX.Element {
    // submit what neither the browser nor the draft refuses, once at a time
    const rest = omit(properties, "form");
    const submit = (event: SubmitEvent & { readonly currentTarget: HTMLFormElement }): void => {
        // leave a submission under way, and what the browser refuses
        event.preventDefault();
        const { currentTarget } = event;
        const isPending = properties.form.status() === "pending";
        if (isPending || !currentTarget.reportValidity() || properties.form.submit()) {
            return;
        }

        // focus what the draft refuses first
        flush();
        currentTarget
            .querySelector<HTMLElement>(
                "[data-slot=field][data-invalid] :is(input:not([type=hidden], [aria-hidden]), select, textarea, button, [tabindex='0'])",
            )
            ?.focus();
    };

    return (
        <FormContext value={properties.form}>
            {renderPart("form", "form", rest, [], {
                novalidate: true,
                get "data-status"() {
                    return properties.form.status();
                },
                onSubmit: submit,
                onReset: () => properties.form.reset(),
            })}
        </FormContext>
    );
}

/** The properties of the error of a form, the native element's attributes included. */
export type FormErrorProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "children"> &
    ElementPartProperties & {
        /** Tell why a submission failed, in general by default. */
        readonly children?: (error: unknown) => JSX.Element;
    };

/** Render why the nearest form's last submission failed, nothing while it has not. */
export function FormError(properties: FormErrorProperties): JSX.Element {
    // read the failure
    const form = useDraft();
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style", "children");

    return (
        <Show when={form.status() === "failed"}>
            <div
                role="alert"
                data-slot="form-error"
                {...rest}
                {...style.attributes([text.sm, styles.error, properties.xstyle], properties.style)}
            >
                {properties.children?.(form.error()) ??
                    locale.render(t`This could not be saved. Try again.`)}
            </div>
        </Show>
    );
}

/** Render the button that submits the nearest form, busy while it does. */
export function FormSubmit(properties: ButtonProperties): JSX.Element {
    const form = useDraft();

    return (
        <Button
            data-slot="form-submit"
            type="submit"
            loading={form.status() === "pending"}
            {...properties}
        />
    );
}

/** Render the button that drops the nearest form's edits, of no use without any. */
export function FormReset(properties: ButtonProperties): JSX.Element {
    const form = useDraft();

    return (
        <Button
            data-slot="form-reset"
            type="reset"
            variant="ghost"
            disabled={!form.isEdited()}
            {...properties}
        />
    );
}

/** The properties of what a form says of its draft, the native element's attributes included. */
export type FormStatusProperties = Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class" | "children"> &
    ElementPartProperties & {
        /** Say where the draft stands, in words by default. */
        readonly children?: (standing: FormStanding) => JSX.Element;
    };

/** Where a form's draft stands: as it was, edited, or as its last submission went. */
export type FormStanding = "clean" | "edited" | "pending" | "saved" | "failed";

/** Render where the nearest form's draft stands, in a line a screen reader hears. */
export function FormStatus(properties: FormStatusProperties): JSX.Element {
    // say what matters most: what runs, then what is not saved yet
    const form = useDraft();
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style", "children");
    const standing = (): FormStanding => {
        const status = form.status();

        return status === "pending" || status === "failed"
            ? status
            : form.isEdited()
              ? "edited"
              : status === "saved"
                ? "saved"
                : "clean";
    };
    const words = (): string => {
        switch (standing()) {
            case "pending":
                return locale.render(t`Saving…`);
            case "failed":
                return locale.render(t`Not saved`);
            case "edited":
                return locale.render(t`Unsaved changes`);
            case "saved":
                return locale.render(t`Saved`);
            case "clean":
                return "";
        }
    };

    return (
        <span
            role="status"
            data-slot="form-status"
            data-standing={standing()}
            {...rest}
            {...style.attributes([text.sm, styles.status, properties.xstyle], properties.style)}
        >
            {properties.children?.(standing()) ?? words()}
        </span>
    );
}

/** What holds navigation within a page, such as `useBlocker`'s answer. */
export interface FormBlocker {
    /** Whether a navigation is held. */
    readonly state: () => "unblocked" | "blocked";
    /** Let the held navigation through. */
    readonly proceed: () => void;
    /** Drop the held navigation and stay. */
    readonly reset: () => void;
}

/** Render the question that keeps a person from leaving the nearest form's edits behind. */
export function FormGuard(properties: {
    /** What holds navigation while there are edits, such as `useBlocker`'s answer. */
    readonly blocker: FormBlocker;
    /** What the question says, that the edits are lost by default. */
    readonly children?: JSX.Element;
}): JSX.Element {
    // ask the browser to hold what leaves the page
    const form = useDraft();
    const locale = useLocale();
    const { blocker } = properties;
    createEffect(
        () => form.isEdited(),
        (isEdited) => {
            if (isEdited) {
                window.addEventListener("beforeunload", block);
            }

            return () => window.removeEventListener("beforeunload", block);
        },
    );

    return (
        <AlertDialog
            open={blocker.state() === "blocked"}
            onOpenChange={(isOpen) => {
                if (!isOpen) {
                    blocker.reset();
                }
            }}
        >
            <AlertDialogContent>
                <AlertDialogHeader>
                    <AlertDialogTitle>{locale.render(t`Leave without saving?`)}</AlertDialogTitle>
                    <AlertDialogDescription>
                        {properties.children ?? locale.render(t`Your changes will be lost.`)}
                    </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogCancel>{locale.render(t`Stay`)}</AlertDialogCancel>
                    <AlertDialogAction variant="destructive" onClick={() => blocker.proceed()}>
                        {locale.render(t`Leave`)}
                    </AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    );
}

/** Read the draft of the nearest form, refusing a part outside one. */
function useDraft(): Draft {
    const form = useContext(FormContext);
    if (form === null) {
        throw new TypeError("a part of a form needs a form around it");
    }

    return form;
}

/** Ask the browser to hold a person who leaves the page. */
function block(event: BeforeUnloadEvent): void {
    event.preventDefault();
}
