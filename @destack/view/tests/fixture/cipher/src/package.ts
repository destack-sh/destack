import { t } from "@destack/locale";
import { definePackage } from "@destack/object";

/** The handle stacks import to install the cipher app. */
export default definePackage({
    title: t`Cipher`,
    description: t`Encrypt notes on the device.`,
    icon: "lock",
    color: "gray",
});
