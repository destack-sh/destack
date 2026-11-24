import eyeIcon from "@destack/icon/phosphor/eye";
import eyeSlashIcon from "@destack/icon/phosphor/eye-slash";
import { t } from "@destack/locale";
import { createControllableSignal, type JSX, omit, useLocale } from "@destack/view";
import { useFieldControl } from "../field/control.ts";
import { Icon } from "../icon/index.ts";
import {
    InputGroup,
    InputGroupAddon,
    InputGroupButton,
    InputGroupInput,
} from "../input-group/index.ts";
import type { InputProperties } from "../input/index.ts";

/** The properties of a password's input, the native input's attributes included. */
export type PasswordFieldProperties = Omit<InputProperties, "type"> & {
    /** Whether the password is shown, which makes showing it controlled. */
    readonly visible?: boolean;
    /** Handle the password being shown or hidden. */
    readonly onVisibleChange?: (isVisible: boolean) => void;
};

/** Render the input of a password or another secret, which a button shows and hides. */
export function PasswordField(properties: PasswordFieldProperties): JSX.Element {
    // show it by the owner's state or its own
    const locale = useLocale();
    const field = useFieldControl();
    const [isVisible, setVisible] = createControllableSignal<boolean>({
        isControlled: () => "visible" in properties,
        value: () => properties.visible === true,
        defaultValue: false,
        onChange: (isNext) => properties.onVisibleChange?.(isNext),
    });
    const rest = omit(properties, "visible", "onVisibleChange", "xstyle");

    return (
        <InputGroup data-slot="password-field" xstyle={properties.xstyle}>
            <InputGroupInput
                autocomplete="current-password"
                autocapitalize="none"
                spellcheck="false"
                {...rest}
                type={isVisible() ? "text" : "password"}
            />
            <InputGroupAddon align="inline-end">
                <InputGroupButton
                    size="icon-sm"
                    aria-label={locale.render(t`Show password`)}
                    aria-pressed={isVisible() ? "true" : "false"}
                    disabled={properties.disabled === true || field?.isDisabled() === true}
                    onClick={() => setVisible(!isVisible())}
                >
                    <Icon icon={isVisible() ? eyeSlashIcon : eyeIcon} />
                </InputGroupButton>
            </InputGroupAddon>
        </InputGroup>
    );
}
