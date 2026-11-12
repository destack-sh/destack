import * as style from "@destack/style";

import { agentPrompt } from "../content/site";
import { InstallList } from "../install/install";

/** Offer the three ways to install across the footer's install column. */
export default function InstallIsland() {
    return <InstallList prompt={agentPrompt} xstyle={styles.wide} />;
}

/** The install island styles. */
const styles = style.create({
    wide: {
        width: "100%",
    },
});
