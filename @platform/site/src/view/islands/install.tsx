import * as style from "@destack/style";
import { createSignal, onSettled } from "@destack/view";

import { agentPrompt } from "../content/site";
import { InstallList } from "../install/install";
import { installPrompt } from "../install/prompt";

/** Offer the three ways to install in the footer's install column, with the page's prompt. */
export default function InstallIsland() {
    // hydrate on the prompt the server rendered, then follow the page's once every island hydrated
    const [isSettled, setIsSettled] = createSignal(false);
    onSettled(() => queueMicrotask(() => setIsSettled(true)));

    return (
        <InstallList prompt={isSettled() ? installPrompt() : agentPrompt} xstyle={styles.wide} />
    );
}

/** The install island styles. */
const styles = style.create({
    wide: {
        width: "100%",
    },
});
