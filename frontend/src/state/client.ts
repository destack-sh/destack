import { graphql, useFragment } from "@/gql";
import { ClientType } from "@/gql/graphql";
import { useAuth } from "@/state/auth";
import { useEditorState } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { WS_CONNECTED } from "@/utils/globals";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { v4 as uuidv4 } from "uuid";
import { onBeforeUnmount, ref, toRef, watch, type Ref, computed } from "vue";

export const CLIENT_NONCE = uuidv4();

function newClientId(): string {
  /* Generates a new statement global id (as in relay) with a new uuid4 */
  const nodeId = uuidv4();
  return btoa(`Client:${nodeId}`);
}

export function getClientColor(clientId: string): string {
  /* Generate a pastelle color for the given client id */

  // Convert the client ID to a numerical seed
  const seed = clientId.split("").reduce((acc, char) => {
    return acc * 31 + char.charCodeAt(0);
  }, 0);

  // Generate a random pastel color based on the seed
  const hue = seed % 360;
  const saturation = 50 + (seed % 30); // Range: 50-80
  const lightness = 70 + (seed % 20); // Range: 70-90

  return `hsl(${hue}, ${saturation}%, ${lightness}%)`;
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
  const projectId = toRef(editor, "currentProjectId");
  const projectVersionId = toRef(editor, "currentProjectVersionId");
  const auth = useAuth();
  const ops = useOperations();

  const localClientId = getOrCreateClientId();
  const deviceName = getDeviceName();
  const browserName = getBrowserName();
  const clientType = ["Windows", "MacOS", "Linux"].includes(deviceName)
    ? ClientType.DesktopBrowser
    : ClientType.MobileBrowser;
  const info = ref({
    id: localClientId,
    type: clientType,
    deviceName,
    browserName,
    nonce: CLIENT_NONCE,
  });

  // upsert client info if logged in
  watch(
    () => [auth.loggedIn.value, projectId.value, projectVersionId.value],
    async () => {
      if (auth.loggedIn.value) {
        await ops.client.upsert(
          localClientId,
          clientType,
          deviceName,
          browserName,
          projectId.value,
          projectVersionId.value
        );
      } else {
        // remove client info
        localStorage.removeItem("client_id");
      }
    }
  );

  // periodically update presence (if logged in and window is focused)
  const interval = setInterval(async () => {
    if (auth.loggedIn.value && document.visibilityState === "visible") {
      await ops.client.updatePresence(true); // silence errors
    }
  }, presenceIntervalMs);
  onBeforeUnmount(close);

  async function close() {
    clearInterval(interval);
    if (auth.loggedIn.value && WS_CONNECTED.value) {
      await ops.client.close();
    }
  }

  return {
    info,
    close,
  };
}

export const useClient = createSharedComposable(_useClient);

export const ClientContentType = graphql(/* GraphQL */ `
  fragment ClientContentType on Client {
    id
    type
    deviceName
    browserName
    user {
      id
      name
      username
    }
    project {
      id
      name
      path
    }
  }
`);

export function useConnectedClients(
  filter: {
    projectId: Ref<string | null>;
    projectVersionId: Ref<string | null>;
    userId: Ref<string | null>;
    inSameOrganizations?: Ref<boolean>;
  },
  options: { live?: boolean; first?: number }
) {
  const { result: clientsResult, subscribeToMore } = useQuery(
    graphql(/* GraphQL */ `
      query connectedClients(
        $projectId: GlobalID
        $projectVersionId: GlobalID
        $userId: GlobalID
        $inSameOrganizations: Boolean!
        $first: Int
      ) {
        clients(
          projectId: $projectId
          projectVersionId: $projectVersionId
          userId: $userId
          inSameOrganizations: $inSameOrganizations
          first: $first
        ) {
          totalCount
          edges {
            node {
              ...ClientContentType
            }
          }
        }
      }
    `),
    {
      projectId: filter.projectId,
      projectVersionId: filter.projectVersionId,
      userId: filter.userId,
      inSameOrganizations: filter.inSameOrganizations,
      first: options.first,
    }
  );

  if (options.live) {
    // TODO @Feature: subscribe to client changes
  }

  const client = useClient();
  const clients = computed(
    () => clientsResult.value?.clients.edges.map((edge: any) => useFragment(ClientContentType, edge.node)) ?? []
  );
  return {
    totalCount: computed(() => clientsResult.value?.clients.totalCount ?? 0),
    clients,
    clientsWithoutSelf: computed(() => clients.value.filter((c) => c.id != client.info.value.id)),
  };
}
