import { ObjectType } from "@destack/object";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { account, space, user } from "@destack/account/object";
import { deviceSetting, setting } from "@destack/setting/object";

/** Every part a scope plays for a page, whose object types the person's device stores together. */
const DEVICE_ROLES = ["user", "account", "space"] as const;

/** The parts a scope plays for a page, each opening its own object types. */
export const DeviceRole = Object.assign(schema.enum(DEVICE_ROLES), {
    /** List the parts a page's scopes may play: the person's own only on their device. */
    played(device: Subject | undefined): DeviceRole[] {
        return DEVICE_ROLES.filter((role) => role !== "user" || device !== undefined);
    },

    /** List the object types some parts open among some types, keyed by type. */
    objects(
        objects: readonly ObjectType[],
        roles: readonly (typeof DEVICE_ROLES)[number][],
    ): Readonly<Record<string, ObjectType>> {
        return Object.fromEntries(
            roles.flatMap((role) =>
                Object.entries(
                    role === "user"
                        ? ObjectType.keyed([setting, deviceSetting, user])
                        : ObjectType.keyed(
                              objects.filter((object) =>
                                  object.livesInType(role === "account" ? account : space),
                              ),
                          ),
                ),
            ),
        );
    },
});
/** The part a scope plays for a page. */
export type DeviceRole = schema.Infer<typeof DeviceRole>;

/** What a page's tab asks its device worker for: the person's objects of the package's views. */
export const DeviceRequest = schema.object({
    /** The device's name, one per installation and person, shared by the installation's tabs. */
    name: schema.string().min(1),
    /** The person signed in. */
    caller: Subject,
    /** The browser's device storing device-scoped local objects. */
    device: Subject.exactOptional(),
    /** The page's origin with its base, which serves the package and the homes. */
    root: schema.string().min(1),
    /** The space of the page's installation, whose store the device connects to. */
    space: schema.string().min(1),
});
/** What a page's tab asks its device worker for. */
export type DeviceRequest = schema.Infer<typeof DeviceRequest>;
