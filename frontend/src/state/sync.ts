import { ClientType } from "@/gql/graphql";
import { useAuth } from "@/state/auth";
import { useEditorState } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { createSharedComposable } from "@vueuse/core";
import { v4 as uuidv4 } from "uuid";
import { onBeforeUnmount, ref, toRef, watch } from "vue";

function newClientId(): string {
  /* Generates a new statement global id (as in relay) with a new uuid4 */
  const nodeId = uuidv4();
  return btoa(`Client:${nodeId}`);
}

export function getBrowserName(): string {
  const userAgent = navigator.userAgent;
  let browserName = "Unknown";

  if (userAgent.indexOf("Chrome") > -1) {
    browserName = "Chrome";
  } else if (userAgent.indexOf("Safari") > -1) {
    browserName = "Safari";
  } else if (userAgent.indexOf("Firefox") > -1) {
    browserName = "Firefox";
  } else if (userAgent.indexOf("MSIE") > -1 || userAgent.indexOf("Trident/") > -1) {
    browserName = "Internet Explorer";
  } else if (userAgent.indexOf("Edge") > -1) {
    browserName = "Edge";
  } else if (userAgent.indexOf("Opera") > -1) {
    browserName = "Opera";
  }

  return browserName;
}

export function getDeviceName(): string {
  const platform = navigator.platform;
  let deviceName = "Unknown";

  if (platform.indexOf("Win") > -1) {
    deviceName = "Windows";
  } else if (platform.indexOf("Mac") > -1) {
    deviceName = "MacOS";
  } else if (platform.indexOf("Linux") > -1) {
    deviceName = "Linux";
  } else if (platform.indexOf("Android") > -1) {
    deviceName = "Android";
  } else if (platform.indexOf("iPhone") > -1) {
    deviceName = "iPhone";
  } else if (platform.indexOf("iPad") > -1) {
    deviceName = "iPad";
  }

  return deviceName;
}

function getOrCreateClientId(): string {
  /* Gets client id from local storage or creates a new one */
  const key = "client_id";
  const existing = localStorage.getItem(key);
  if (existing) {
    return existing;
  } else {
    const newId = newClientId();
    localStorage.setItem(key, newId);
    return newId;
  }
}

function _useClient(presenceIntervalMs = 10000) {
  const editor = useEditorState();
  const projectVersionId = toRef(editor, "currentProjectVersionId");
  const auth = useAuth();
  const ops = useOperations();

  const localClientId = getOrCreateClientId();
  const deviceName = getDeviceName();
  const browserName = getBrowserName();
  const clientType = ["Windows", "MacOS", "Linux"].includes(deviceName)
    ? ClientType.DesktopBrowser
    : ClientType.MobileBrowser;
  const clientInfo = ref({
    id: localClientId,
    type: clientType,
    deviceName,
    browserName,
  });

  // upsert client info if logged in
  watch(
    () => [auth.loggedIn.value, projectVersionId.value],
    async () => {
      if (auth.loggedIn.value) {
        await ops.client.upsert(localClientId, clientType, deviceName, browserName, projectVersionId.value);
      } else {
        // remove client info
        localStorage.removeItem("client_id");
      }
    }
  );

  // periodically update presence
  const interval = setInterval(async () => {
    if (auth.loggedIn.value) {
      await ops.client.updatePresence();
    }
  }, presenceIntervalMs);
  onBeforeUnmount(() => {
    clearInterval(interval);
  });

  return {
    clientInfo,
  };
}

export const useClient = createSharedComposable(_useClient);

function _useProjectSync() {
  const editor = useEditorState();
  const projectVersionId = toRef(editor, "currentProjectVersionId");
  // TODO @Incomplete: implement basic sync
}

export const useProjectSync = createSharedComposable(_useProjectSync);
