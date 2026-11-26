import { principal } from "@destack/access";
import { t } from "@destack/locale";
import { PackageVersion } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { defineView } from "../declare/view.ts";
import { DeviceRole } from "./request.ts";

/** The notes package whose view the device serves. */
const notes = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000f1",
    name: "@example/notes",
    version: "2026.9.0",
});

/** The notes view, opening no object types of its own. */
const view = defineView(
    { name: "notes", title: t`Notes`, component: () => Promise.reject(new Error("unloaded")) },
    { package: notes },
);

test("store no person's settings on a visitor's device, and the person's on theirs", () => {
    // list the stored types without a device, then on the person's device
    const device = principal.device.reference(
        "universe",
        schema.identifier("device").parse("device-01996ab0-0000-7000-8000-0000000000f3"),
    );

    // expect the person's settings, device settings and profile on their device alone
    expect({
        visitor: typeNames(DeviceRole.objects(view.objects, DeviceRole.played(undefined))),
        person: typeNames(DeviceRole.objects(view.objects, DeviceRole.played(device))),
    }).toEqual({ visitor: [], person: ["setting", "device-setting", "user"] });
});

/** Read the names of some stored object types. */
function typeNames(stored: ReturnType<typeof DeviceRole.objects>): string[] {
    return Object.values(stored).map((type) => type.name);
}
