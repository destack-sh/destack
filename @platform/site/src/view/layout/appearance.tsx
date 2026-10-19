import * as style from "@destack/style";
import { Icon } from "@destack/icon";
import moon from "@destack/icon/phosphor/moon";
import sun from "@destack/icon/phosphor/sun";
import { Button } from "@destack/ui/button";
import type { LocalAppearance } from "@destack/view/document";

/** Switch between light and dark, following the system until switched, kept for next visit. */
export function AppearanceToggle(properties: { appearance: LocalAppearance }) {
    // read the appearance a switch turns to
    const target = () => (properties.appearance.resolvedAppearance() === "dark" ? "light" : "dark");

    return (
        <Button
            variant="ghost"
            size="icon"
            aria-label={`Switch to the ${target()} appearance`}
            onClick={() => properties.appearance.setAppearance(target())}
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
