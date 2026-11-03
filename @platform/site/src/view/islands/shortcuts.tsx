import { useEnvironment } from "@destack/view";
import { Shortcut } from "@destack/object";
import { isCommandPlatform } from "@destack/view/palette";
import { createShortcut } from "@destack/view/primitives/keyboard";
import { destinations, isExternal } from "../layout/navigation";

/** Open each destination on its key combination, leaving the site in a new tab. */
export default function Shortcuts() {
    const environment = useEnvironment();
    for (const destination of destinations) {
        createShortcut(
            Shortcut.keys(destination.keybinding, isCommandPlatform()),
            () => {
                if (isExternal(destination)) {
                    window.open(destination.href, "_blank", "noopener");
                } else {
                    environment.navigation.assign(destination.href);
                }
            },
            { preventDefault: true, ignoreWithinInputs: true, anyOrder: true },
        );
    }

    return null;
}
