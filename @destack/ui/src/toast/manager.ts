import { type Accessor, createSignal, type Setter } from "@destack/view";

/** The kind of a toast, which picks its icon. */
export type ToastType = "default" | "success" | "error" | "warning" | "info" | "loading";

/** The button a toast shows beside its text. */
export interface ToastButton {
    /** The text of the button. */
    readonly label: string;
    /** Handle a click, after which the toast leaves. */
    readonly onClick: (event: MouseEvent) => void;
}

/** The options of a toast. */
export interface ToastOptions {
    /** The id of the toast, which a later toast with the same id replaces. */
    readonly id?: string;
    /** The text under the title. */
    readonly description?: string;
    /** The time the toast shows in milliseconds, without end for an error or loading work. */
    readonly duration?: number;
    /** The button, such as undo. */
    readonly action?: ToastButton;
    /** Handle the toast leaving, by a person or on its own. */
    readonly onDismiss?: () => void;
}

/** A toast in the toaster. */
export interface Toast extends ToastOptions {
    /** The id of the toast. */
    readonly id: string;
    /** The kind of the toast. */
    readonly type: ToastType;
    /** The main text. */
    readonly title: string;
}

/** The texts a promise's toast shows while pending, once fulfilled and once rejected. */
export interface ToastPromiseMessages<Value> {
    /** The text while the promise is pending. */
    readonly loading: string;
    /** The text once the promise fulfils, or the text written from its value. */
    readonly success: string | ((value: Value) => string);
    /** The text once the promise rejects, or the text written from its reason. */
    readonly error: string | ((reason: unknown) => string);
}

/** The toasts of a toaster, after Base UI's toast manager. */
export class ToastManager {
    /** The toasts, newest first, the dismissed ones until they have slid out. */
    readonly toasts: Accessor<readonly Toast[]>;
    /** The ids of the dismissed toasts still sliding out. */
    readonly leaving: Accessor<ReadonlySet<string>>;
    /** Replace the toasts. */
    readonly #setToasts: Setter<readonly Toast[]>;
    /** Replace the ids of the toasts sliding out. */
    readonly #setLeaving: Setter<ReadonlySet<string>>;
    /** The number of toasts shown so far, which numbers the next toast without an id. */
    #count = 0;

    /** Create the toasts of a toaster. */
    constructor() {
        // start without toasts
        const [toasts, setToasts] = createSignal<readonly Toast[]>([], { ownedWrite: true });
        const [leaving, setLeaving] = createSignal<ReadonlySet<string>>(new Set(), {
            ownedWrite: true,
        });
        this.toasts = toasts;
        this.leaving = leaving;
        this.#setToasts = setToasts;
        this.#setLeaving = setLeaving;
    }

    /** Show a toast, returning its id. */
    show(title: string, options?: ToastOptions): string {
        return this.#add(title, "default", options);
    }

    /** Show a toast of a success, returning its id. */
    success(title: string, options?: ToastOptions): string {
        return this.#add(title, "success", options);
    }

    /** Show a toast of an error, which stays until dismissed, returning its id. */
    error(title: string, options?: ToastOptions): string {
        return this.#add(title, "error", options);
    }

    /** Show a toast of a warning, returning its id. */
    warning(title: string, options?: ToastOptions): string {
        return this.#add(title, "warning", options);
    }

    /** Show a toast of information, returning its id. */
    info(title: string, options?: ToastOptions): string {
        return this.#add(title, "info", options);
    }

    /** Show a toast of work in progress, which stays until replaced or dismissed. */
    loading(title: string, options?: ToastOptions): string {
        return this.#add(title, "loading", options);
    }

    /** Show a loading toast while a promise is pending, and its outcome once it settles. */
    promise<Value>(
        promise: Promise<Value>,
        messages: ToastPromiseMessages<Value>,
        options?: ToastOptions,
    ): Promise<Value> {
        // turn the loading toast into the outcome
        const id = this.loading(messages.loading, options);
        const { success, error } = messages;
        void promise.then(
            (value) =>
                this.success(typeof success === "string" ? success : success(value), {
                    ...options,
                    id,
                }),
            (reason: unknown) =>
                this.error(typeof error === "string" ? error : error(reason), { ...options, id }),
        );

        return promise;
    }

    /** Dismiss a toast, or every toast without an id, sliding each out and telling it. */
    dismiss(id?: string): void {
        // leave a toast alone that is already sliding out
        const dismissed = this.toasts().filter(
            (entry) => (id === undefined || entry.id === id) && !this.leaving().has(entry.id),
        );
        this.#setLeaving((ids) => new Set([...ids, ...dismissed.map((entry) => entry.id)]));
        for (const entry of dismissed) {
            entry.onDismiss?.();
        }
    }

    /** Remove a dismissed toast once it has slid out, unless it was shown again meanwhile. */
    remove(id: string): void {
        // leave a toast shown again in place
        if (!this.leaving().has(id)) {
            return;
        }
        this.#setLeaving((ids) => new Set([...ids].filter((each) => each !== id)));
        this.#setToasts((toasts) => toasts.filter((entry) => entry.id !== id));
    }

    /** Add a toast first, or replace the one with the same id in place, returning its id. */
    #add(title: string, type: ToastType, options: ToastOptions = {}): string {
        // take the id given, else the next one
        this.#count += 1;
        const id = options.id ?? `toast-${String(this.#count)}`;
        const shown: Toast = { ...options, id, type, title };
        this.#setLeaving((ids) => new Set([...ids].filter((each) => each !== id)));
        this.#setToasts((toasts) =>
            toasts.some((entry) => entry.id === id)
                ? toasts.map((entry) => (entry.id === id ? shown : entry))
                : [shown, ...toasts],
        );

        return id;
    }
}
