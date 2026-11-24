import searchIcon from "@destack/icon/phosphor/magnifying-glass";
import xIcon from "@destack/icon/phosphor/x";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { createControllableSignal, type JSX, omit, Show, untrack, useLocale } from "@destack/view";
import { readText, useFieldControl, useFieldValue } from "../field/control.ts";
import { handleFormReset } from "../form-input/index.ts";
import { Icon } from "../icon/index.ts";
import {
    InputGroup,
    InputGroupAddon,
    InputGroupButton,
    InputGroupInput,
} from "../input-group/index.ts";
import type { InputProperties } from "../input/index.ts";
import { mergeProperties } from "../part/index.ts";

/** The styles of a search field. */
const styles = style.create({
    input: {
        "::-webkit-search-cancel-button": { display: "none" },
    },
});

/** The properties of a search field, the native input's attributes included. */
export type SearchFieldProperties = Omit<InputProperties, "type" | "value" | "onSubmit"> & {
    /** The text searched for, which makes the field controlled. */
    readonly value?: string;
    /** The text the field starts from when uncontrolled. */
    readonly defaultValue?: string;
    /** Handle the text changing. */
    readonly onValueChange?: (value: string) => void;
    /** Handle the text a person searches for by Enter. */
    readonly onSubmit?: (value: string) => void;
    /** Handle the text being cleared, by the button or Escape. */
    readonly onClear?: () => void;
};

/** Render a field to search by, cleared by its button or Escape. */
export function SearchField(given: SearchFieldProperties): JSX.Element {
    // edit the value of the field's form
    const properties = useFieldValue(given, {
        value: "value",
        change: "onValueChange",
        read: readText,
    });

    // follow the controlled text or the field's own
    const locale = useLocale();
    const field = useFieldControl();
    const [value, setValue] = createControllableSignal<string>({
        isControlled: () => properties.value !== undefined,
        value: () => properties.value ?? "",
        defaultValue: untrack(() => properties.defaultValue) ?? "",
        onChange: (next) => properties.onValueChange?.(next),
    });
    let input: HTMLInputElement | undefined;
    const form = (): HTMLFormElement | null | undefined => input?.form;
    const rest = omit(
        properties,
        "value",
        "defaultValue",
        "onValueChange",
        "onSubmit",
        "onClear",
        "xstyle",
    );
    handleFormReset(
        () => form(),
        () => setValue(properties.defaultValue ?? ""),
    );
    const clear = (): void => {
        setValue("");
        properties.onClear?.();
        input?.focus();
    };

    // leave a field that takes no input
    const isEditable = (): boolean =>
        properties.disabled !== true &&
        properties.readonly !== true &&
        field?.isDisabled() !== true;

    // take the keys of a search
    const keys = (event: KeyboardEvent): void => {
        // search, or clear what there is before Escape closes something around
        if (event.isComposing) {
            return;
        } else if (event.key === "Enter") {
            properties.onSubmit?.(value());
        } else if (event.key === "Escape" && value() !== "" && isEditable()) {
            event.preventDefault();
            event.stopPropagation();
            clear();
        }
    };

    return (
        <InputGroup data-slot="search-field" xstyle={properties.xstyle}>
            <InputGroupAddon>
                <Icon icon={searchIcon} />
            </InputGroupAddon>
            <InputGroupInput
                enterkeyhint="search"
                placeholder={locale.render(t`Search`)}
                aria-label={field === null ? locale.render(t`Search`) : undefined}
                {...mergeProperties(
                    {
                        ref: (element: HTMLInputElement) => (input = element),
                        onInput: (
                            event: InputEvent & { readonly currentTarget: HTMLInputElement },
                        ) => setValue(event.currentTarget.value),
                        onKeyDown: keys,
                    },
                    rest,
                )}
                type="search"
                value={value()}
                xstyle={styles.input}
            />
            <Show when={value() !== "" && isEditable()}>
                <InputGroupAddon align="inline-end">
                    <InputGroupButton
                        size="icon-sm"
                        tabindex={-1}
                        aria-label={locale.render(t`Clear`)}
                        onClick={clear}
                    >
                        <Icon icon={xIcon} />
                    </InputGroupButton>
                </InputGroupAddon>
            </Show>
        </InputGroup>
    );
}
