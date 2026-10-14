import * as style from "@destack/style";
import { Icon } from "@destack/icon";
import moon from "@destack/icon/phosphor/moon";
import sun from "@destack/icon/phosphor/sun";
import { Button } from "@destack/ui/button";
import { createAppearance } from "@destack/view/document";
import { createEventListener } from "@destack/view/primitives/event-listener";

import { commandEvents } from "./command";

/** Switch between light and dark, following the system until switched, kept for next visit. */
export function AppearanceToggle() {
    // read the appearance, and the one a switch turns to
    const { setAppearance, resolvedAppearance } = createAppearance();
    const target = () => (resolvedAppearance() === "dark" ? "light" : "dark");

    // switch too when the command palette asks
    createEventListener(
        () => document,
        commandEvents.toggleAppearance,
        () => setAppearance(target()),
    );

    return (
        <Button
            variant="ghost"
            size="icon"
            aria-label={`Switch to the ${target()} appearance`}
            onClick={() => setAppearance(target())}
        >
            <Icon icon={target() === "dark" ? moon : sun} xstyle={styles.icon} />
        </Button>
    );
}

/** The toggle icon, at the size of the top bar's other tools. */
const styles = style.create({
    icon: {
        width: "20px",
        height: "20px",
    },
});
