import { createSignal } from "@destack/view";

import { agentPrompt } from "../content/site";

// NOTE #Architecture: page-wide state, so the footer's install island offers the homepage's picks
/** The agent prompt every install control on the page offers, and how the homepage rewrites it. */
export const [installPrompt, setInstallPrompt] = createSignal(agentPrompt);
