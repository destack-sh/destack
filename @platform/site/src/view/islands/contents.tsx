import { Icon } from "@destack/icon";
import caretDown from "@destack/icon/phosphor/caret-down";
import { text } from "@destack/theme/text";
import { Popover, PopoverContent, PopoverTrigger } from "@destack/ui/popover";
import { createSignal, type JSX } from "@destack/view";
import * as style from "@destack/style";
import { screen } from "../layout/screen.stylex";

/** Open the article's contents from a narrow toolbar, closing once a link is followed. */
export default function Contents(properties: { readonly children?: JSX.Element }) {
    const [isOpen, setIsOpen] = createSignal(false);

    return (
        <Popover open={isOpen()} onOpenChange={setIsOpen}>
            <PopoverTrigger variant="ghost" size="sm" xstyle={styles.menu}>
                Contents
                <Icon icon={caretDown} />
            </PopoverTrigger>
            <PopoverContent side="bottom" align="start" xstyle={text.sm}>
                <div
                    {...style.attrs(styles.menuBody)}
                    onClick={(event) => {
                        // close once a link in the contents is followed
                        if (event.target instanceof Element && event.target.closest("a")) {
                            setIsOpen(false);
                        }
                    }}
                >
                    {properties.children}
                </div>
            </PopoverContent>
        </Popover>
    );
}

/** The contents menu's styles. */
const styles = style.create({
    menu: {
        minWidth: 0,
        display: { default: "inline-flex", [screen.reader]: "none" },
    },
    menuBody: {
        display: "grid",
        gap: "1.25rem",
        maxHeight: "min(32rem, 65svh)",
        overflowY: "auto",
        overscrollBehavior: "contain",
        width: "min(22rem, calc(100vw - 2rem))",
    },
});
