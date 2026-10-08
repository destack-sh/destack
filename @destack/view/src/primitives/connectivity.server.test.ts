import { expect, test } from "@destack/test";
import {
    createConnectivitySignal,
    createNetworkInformation,
    useConnectivitySignal,
    useNetworkInformation,
} from "./connectivity.ts";

test("read online with unknown quality on the server", () => {
    const network = createNetworkInformation();

    expect([
        createConnectivitySignal()(),
        useConnectivitySignal()(),
        network.online(),
        network.effectiveType(),
        useNetworkInformation().rtt(),
    ]).toEqual([true, true, true, undefined, undefined]);
});
