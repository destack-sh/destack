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
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of a password field besides its declared ones: its input's other attributes. */
export type PasswordFieldAttributes = Omit<
    InputProperties,
    "type" | "variant" | keyof typeof InputGroupInput.properties
> & {
    /** Whether the password is shown, which makes showing it controlled. */
    readonly visible?: boolean;
    /** Handle the password being shown or hidden. */
    readonly onVisibleChange?: (isVisible: boolean) => void;
};

/** Render the input of a password or another secret, which a button shows and hides. */
export const PasswordField = defineComponent({
    name: "passwordField",
    properties: InputGroupInput.properties,
    render(
        properties: RenderProperties<PasswordFieldAttributes, typeof InputGroupInput.properties>,
    ): JSX.Element {
        // show it by the owner's state or its own
        const locale = useLocale();
        const fieldControl = useFieldControl();
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
                        isIconOnly
                        size="sm"
                        aria-label={locale.render(t.context("action")`Show password`)}
                        aria-pressed={isVisible() ? "true" : "false"}
                        disabled={
                            properties.disabled === true || fieldControl?.isDisabled() === true
                        }
                        onClick={() => setVisible(!isVisible())}
                    >
                        <Icon icon={isVisible() ? eyeSlashIcon : eyeIcon} />
                    </InputGroupButton>
                </InputGroupAddon>
            </InputGroup>
        );
    },
});

/** The properties of a password field, the native input's attributes included. */
export type PasswordFieldProperties = ComponentProperties<typeof PasswordField>;
