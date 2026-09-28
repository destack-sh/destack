import { defineServiceConnection } from "@destack/service/declare";
import { settingService } from "@destack/setting/service";

/** Connection to the setting authority selected by the host. */
export const settings = defineServiceConnection("settings", settingService);
