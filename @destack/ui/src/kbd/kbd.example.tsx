import type { JSX } from "@solidjs/web";
import { Kbd, KbdGroup } from "./kbd.tsx";

/** Show the shortcut that opens the command menu. */
export function KbdExample(): JSX.Element {
    return (
        <p>
            Open the command menu with{" "}
            <KbdGroup>
                <Kbd>⌘</Kbd>
                <Kbd>K</Kbd>
            </KbdGroup>
        </p>
    );
}
