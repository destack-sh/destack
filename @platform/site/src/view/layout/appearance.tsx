import * as style from "@destack/style";
import { Icon } from "@destack/icon";
import moon from "@destack/icon/phosphor/moon";
import sun from "@destack/icon/phosphor/sun";
import { Button } from "@destack/ui/button";
import { createAppearance } from "@destack/view/document";

/** Switch between the light and dark appearance, following the system until switched and kept for the next visit. */
export function AppearanceToggle() {
    const { setAppearance, scheme } = createAppearance();
    const target = () => (scheme() === "dark" ? "light" : "dark");

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
