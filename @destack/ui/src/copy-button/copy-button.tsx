import check from "@destack/icon/phosphor/check";
import copyIcon from "@destack/icon/phosphor/copy";
import warning from "@destack/icon/phosphor/warning";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { swatch } from "@destack/theme/tokens.stylex";
import { Icon } from "@destack/ui/icon";
import { createSignal, type JSX, omit, Show, useLocale } from "@destack/view";
import { writeClipboard } from "@destack/view/primitives/clipboard";
import { Timeout } from "@destack/view/primitives/timer";
import { Button, type ButtonProperties } from "../button/index.ts";
import { Tooltip, TooltipContent, TooltipTrigger } from "../tooltip/index.ts";
import { visuallyHiddenStyle } from "../visually-hidden/index.ts";

/** How long the button tells what came of a press, long enough to read it. */
const TOLD_MILLISECONDS = 3000;

/** The styles of what came of a press. */
const styles = style.create({
    copied: { color: swatch.successText },
    refused: { color: swatch.destructiveText },
});

/** The properties of a button that copies, with the button's own. */
export interface CopyButtonProperties extends Omit<ButtonProperties, "onClick"> {
    /** The text it copies. */
    readonly value: string;
    /** What it copies, such as a link, shown as a tooltip and told to assistive technology. */
    readonly label?: string;
}

/** What came of a press: nothing yet, the text was copied, or the browser refused. */
type Outcome = "none" | "copied" | "refused";

/** Render a button that copies text and says so for a while. */
export function CopyButton(properties: CopyButtonProperties): JSX.Element {
    // say what came of a press for a while, again from the start at each press
    const locale = useLocale();
    const timeout = new Timeout();
    const [isOpen, setOpen] = createSignal(false, { ownedWrite: true });
    const [outcome, setOutcome] = createSignal<Outcome>("none", { ownedWrite: true });
    const rest = omit(properties, "value", "label", "children", "variant", "size");
    const tell = (next: Outcome): void => {
        setOutcome(next);
        timeout.set(() => setOutcome("none"), TOLD_MILLISECONDS);
    };
    const copy = (): void => {
        void writeClipboard(properties.value).then(
            () => tell("copied"),
            () => tell("refused"),
        );
    };
    const isAlone = (): boolean => properties.children === undefined;
    const label = (): string => properties.label ?? locale.render(t.context("action")`Copy`);
    const told = (): string =>
        outcome() === "none"
            ? ""
            : locale.render(
                  outcome() === "copied"
                      ? t.context("option")`Copied`
                      : t.context("option")`Not copied`,
              );
    const button = (attributes: JSX.ButtonHTMLAttributes<HTMLButtonElement>): JSX.Element => (
        <Button
            {...attributes}
            data-slot="copy-button"
            data-outcome={outcome()}
            aria-label={isAlone() ? label() : undefined}
            {...rest}
            variant={properties.variant ?? "ghost"}
            size={properties.size ?? "default"}
            isIconOnly={isAlone()}
            onClick={copy}
        >
            <Show when={outcome() !== "none"} fallback={<Icon icon={copyIcon} />}>
                <Icon
                    icon={outcome() === "copied" ? check : warning}
                    xstyle={outcome() === "copied" ? styles.copied : styles.refused}
                />
            </Show>
            {properties.children}
        </Button>
    );

    return (
        <>
            <Show when={isAlone()} fallback={button({})}>
                <Tooltip open={isOpen() || outcome() !== "none"} onOpenChange={setOpen}>
                    <TooltipTrigger render={(attributes) => button(attributes)} />
                    <TooltipContent>{told() === "" ? label() : told()}</TooltipContent>
                </Tooltip>
            </Show>
            <span role="status" {...style.attrs(visuallyHiddenStyle())}>
                {told()}
            </span>
        </>
    );
}
