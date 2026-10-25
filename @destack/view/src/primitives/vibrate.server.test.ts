import { testHost } from "@destack/test/host";
import { BrowserEnvironment } from "@destack/environment/browser";
import { expect, test } from "@destack/test";
import { createComponent } from "solid-js";
import { renderToString } from "@solidjs/web";
import { EnvironmentContext } from "../page/view.ts";
import { createPulse, createVibrate } from "./vibrate.ts";

test("follow a vibration and a pulse on the server without touching a motor", () => {
    // create both in a server render, which never starts the motor
    const read: boolean[] = [];
    renderToString(() =>
        createComponent(EnvironmentContext, {
            value: BrowserEnvironment.server(testHost()),
            get children() {
                read.push(createVibrate(200).vibrating(), createPulse(2).pulsing());

                return "";
            },
        }),
    );

    expect(read).toEqual([false, false]);
});
