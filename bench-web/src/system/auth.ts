import type { BadgeData, ClientData } from "@/proto/wire";
import { useStorage } from "@vueuse/core";
import { v4 } from "uuid";

type BadgeInfo = Pick<BadgeData, "id" | "key" | "password">;
type ClientInfo = Pick<ClientData, "deviceName" | "browserName"> & { nonce: string };

const BROWSER_NAME = getBrowserName();
const DEVICE_NAME = getDeviceName();
const nonce = v4();
const authState = {
  clientAccess: useStorage<{ id: string | null; token: string | null }>("client", {
    id: null,
    token: null,
  }),
  badgesById: useStorage<{ [id: string]: BadgeInfo }>("badges", {}),

  get isAuthenticated() {
    return this.clientAccess.value.token !== null;
  },

  get clientInfo(): ClientInfo {
    return {
      deviceName: BROWSER_NAME,
      browserName: DEVICE_NAME,
      nonce,
    };
  },

  get badges() {
    return Object.values(this.badgesById.value);
  },

  onLoggedIn(clientId: string, clientAccessToken: string) {
    authState.clientAccess.value = { id: clientId, token: clientAccessToken };
  },

  onLoggedOut() {
    authState.clientAccess.value = { id: null, token: null };
  },
};

export default authState;

function getBrowserName() {
  /** Gets the clients browser name and version */
  const userAgent = navigator.userAgent;
  let browserName = "Unknown";
  let version = "Unknown";

  if (userAgent.match(/chrome|chromium|crios/i)) {
    browserName = "Chrome";
    version = userAgent.match(/(chrome|chromium|crios)\/(\d+)/i)?.[2] ?? "Unknown";
  } else if (userAgent.match(/firefox|fxios/i)) {
    browserName = "Firefox";
    version = userAgent.match(/(firefox|fxios)\/(\d+)/i)?.[2] ?? "Unknown";
  } else if (userAgent.match(/safari/i) && !userAgent.match(/chrome|chromium|crios/i)) {
    browserName = "Safari";
    version = userAgent.match(/version\/(\d+)/i)?.[1] ?? "Unknown";
  } else if (userAgent.match(/opr\//i)) {
    browserName = "Opera";
    version = userAgent.match(/opr\/(\d+)/i)?.[1] ?? "Unknown";
  } else if (userAgent.match(/edg/i)) {
    browserName = "Edge";
    version = userAgent.match(/edg\/(\d+)/i)?.[1] ?? "Unknown";
  }

  // Clean non-alphabetical chars from browser name if needed
  browserName = browserName.replace(/[^a-zA-Z ]/g, "");

  return `${browserName} ${version}`;
}

function getDeviceName(): string {
  /** Gets the clients device name */
  const userAgent = navigator.userAgent;
  let deviceName = "Unknown";

  if (/android/i.test(userAgent)) {
    deviceName = "Android";
    const match = userAgent.match(/; (\w+)? Build\//);
    if (match && match[1]) {
      deviceName = match[1].replace(";", "") + " (Android)";
    }
  } else if (/iPad|iPhone|iPod/.test(userAgent) && !(window as any).MSStream) {
    deviceName = userAgent.match(/iPad|iPhone|iPod/)?.[0] ?? "Unknown";
  } else if (/Windows NT/.test(userAgent)) {
    deviceName = "Windows";
  } else if (/Macintosh/.test(userAgent)) {
    deviceName = "MacOS";
  } else if (/Linux/.test(userAgent)) {
    deviceName = "Linux";
  }

  return deviceName;
}
