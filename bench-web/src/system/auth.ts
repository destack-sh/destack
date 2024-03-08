import type { BadgeData, ClientData, UserData } from "@/proto/wire";
import { getBrowserName, getBrowserVersion, getDeviceType, getOperatingSystem } from "@/utils/client";
import { useStorage } from "@vueuse/core";
import { v4 } from "uuid";

type UserInfo = Pick<UserData, "id" | "email" | "name" | "icon">;
type BadgeInfo = Pick<BadgeData, "id" | "key" | "password">;
type ClientInfo = Pick<
  ClientData,
  "deviceName" | "deviceType" | "operatingSystem" | "browserName" | "browserVersion"
> & { nonce: string };

const isOpera = !!(window as any).opera;

export const DEVICE_TYPE = getDeviceType(window.navigator.userAgent);
export const BROWSER_NAME = getBrowserName(window.navigator.userAgent, window.navigator.vendor, isOpera);
export const BROWSER_VERSION = getBrowserVersion(window.navigator.userAgent, window.navigator.vendor, isOpera)?.toString();
export const OPERATING_SYSTEM = getOperatingSystem(window);
export const nonce = v4();

const auth = {
  userInfo: useStorage<UserInfo | null>("userInfo", null),
  clientAccess: useStorage<{ id: string | null; token: string | null }>("clientAccess", {
    id: null,
    token: null,
  }),
  badgesById: useStorage<{ [id: string]: BadgeInfo }>("badges", {}),

  get isAuthenticated() {
    return this.clientAccess.value.token !== null;
  },

  get clientInfo(): ClientInfo {
    return {
      deviceType: DEVICE_TYPE,
      operatingSystem: OPERATING_SYSTEM,
      browserName: BROWSER_NAME,
      browserVersion: BROWSER_VERSION,
      nonce,
    };
  },

  get badges() {
    return Object.values(this.badgesById.value);
  },

  onLoggedIn(clientId: string, clientAccessToken: string) {
    auth.clientAccess.value = { id: clientId, token: clientAccessToken };
  },

  onLoggedOut() {
    auth.clientAccess.value = { id: null, token: null };
  },
};

export default auth;
