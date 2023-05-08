import { graphql, useFragment } from "@/gql";
import { ClientType } from "@/gql/graphql";
import { useAuth } from "@/state/auth";
import { useEditorState } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { getUpdatedConnectionQuery } from "@/utils/connection";
import { toValueRef, wrapValueRefs } from "@/utils/functools";
import { WS_CONNECTED } from "@/utils/globals";
import { useApolloClient, useQuery } from "@vue/apollo-composable";
import { createSharedComposable, useDebounceFn } from "@vueuse/core";
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

function _useClient(presenceIntervalMs = 15000) {
  const editor = useEditorState();
  const projectId = toValueRef(toRef(editor, "currentProjectId"));
  const projectVersionId = toValueRef(toRef(editor, "currentProjectVersionId"));
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

  async function _upsertInfo() {
    await ops.client.upsert(
      localClientId,
      clientType,
      deviceName,
      browserName,
      projectId.value,
      projectVersionId.value,
      editor.focusedFileId,
      editor.focusedElementType == "Statement" ? editor.focusedElementId : null,
      null,
      null,
      null
    );
  }

  const _upsertInfoDebounced = useDebounceFn(_upsertInfo, 500, { maxWait: 2500 });

  // upsert client info if logged in
  const focusedFileId = toValueRef(toRef(editor, "focusedFileId"));
  const focusedElementId = toValueRef(toRef(editor, "focusedElementId"));
  watch(
    () => [auth.loggedIn.value, projectId.value, projectVersionId.value, focusedFileId.value, focusedElementId.value],
    async () => {
      if (auth.loggedIn.value) {
        await _upsertInfoDebounced();
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
      email
    }
    project {
      id
      name
    }
    file {
      id
      name
    }
    statement {
      id
      name
    }
    lastSeenAt
    closedAt
    active
    present
  }
`);

// :ClientTimeouts
export const CLIENT_ACTIVE_TIMEOUT_SECONDS = 60 * 1; // 1 minute
export const CLIENT_PRESENT_TIMEOUT_SECONDS = 60 * 60; // 1 hour

export function useConnectedClients(
  filter: {
    projectId: Ref<string | null>;
    projectVersionId: Ref<string | null>;
    userId: Ref<string | null>;
    inSameOrganizations?: Ref<boolean>;
    active: Ref<boolean | null>;
    present: Ref<boolean | null>;
  },
  options: { live?: boolean; first?: number }
) {
  filter = wrapValueRefs(filter); // rewrap to trigger only if value really changed
  const { result: clientsResult, subscribeToMore } = useQuery(
    graphql(/* GraphQL */ `
      query connectedClients(
        $projectId: GlobalID
        $projectVersionId: GlobalID
        $userId: GlobalID
        $inSameOrganizations: Boolean!
        $first: Int
        $active: Boolean
      ) {
        clients(
          projectId: $projectId
          projectVersionId: $projectVersionId
          userId: $userId
          inSameOrganizations: $inSameOrganizations
          first: $first
          active: $active
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
      active: filter.active,
      present: filter.present,
      first: options.first,
    }
  );

  if (options.live) {
    subscribeToMore({
      document: graphql(/* GraphQL */ `
        subscription clientsChanged($projectId: GlobalID, $projectVersionId: GlobalID) {
          clientsChanged(projectId: $projectId, projectVersionId: $projectVersionId) {
            ...ClientContentType
          }
        }
      `),
      variables: {
        projectId: filter.projectId,
        projectVersionId: filter.projectVersionId,
      },
      updateQuery: (prev, { subscriptionData }) => {
        if (!subscriptionData.data) return prev;
        const client = useFragment(ClientContentType, subscriptionData.data.clientsChanged);
        return {
          clients: getUpdatedConnectionQuery(client, prev.clients),
        };
      },
    });
  }

  const client = useClient();
  const clients = computed(
    () =>
      clientsResult.value?.clients.edges
        .map((edge: any) => useFragment(ClientContentType, edge.node))
        .sort((a, b) => (a.id < b.id ? -1 : 1)) ?? []
  );

  // periodically update active/present status for clients in cache (we don't always get notified on disconnect)
  const { client: apolloClient } = useApolloClient();
  const interval = setInterval(async () => {
    const fragment = graphql(/* GraphQL */ `
      fragment ClientStatus on Client {
        id
        lastSeenAt
        closedAt
        active
        present
      }
    `);
    const activeCutoff = new Date(Date.now() - CLIENT_ACTIVE_TIMEOUT_SECONDS * 1000).toISOString();
    const presentCutoff = new Date(Date.now() - CLIENT_PRESENT_TIMEOUT_SECONDS * 1000).toISOString();
    clients.value.forEach((c) => {
      apolloClient.cache.updateFragment({ id: `Client:${c.id}`, fragment }, (prev: any) => ({
        ...prev,
        active: !(prev.closedAt && prev.closedAt >= prev.lastSeenAt) && prev.lastSeenAt >= activeCutoff,
        present: !(prev.closedAt && prev.closedAt >= prev.lastSeenAt) && prev.lastSeenAt >= presentCutoff,
      }));
    }, 10 * 1000);
  });
  onBeforeUnmount(() => clearInterval(interval));

  return {
    totalCount: computed(() => clientsResult.value?.clients.totalCount ?? 0),
    clients,
    clientsWithoutSelf: computed(() => clients.value.filter((c) => c.id != client.info.value.id)),
    activeClients: computed(() => clients.value.filter((c) => c.active)),
    activeClientsWithoutSelf: computed(() => clients.value.filter((c) => c.active && c.id != client.info.value.id)),
    presentClients: computed(() => clients.value.filter((c) => c.present)),
    presentClientsWithoutSelf: computed(() => clients.value.filter((c) => c.present && c.id != client.info.value.id)),
  };
}

function _useCurrentClients() {
  const editor = useEditorState();
  const clients = useConnectedClients(
    {
      projectId: toRef(editor, "currentProjectId"),
      projectVersionId: toRef(editor, "currentProjectVersionId"),
      userId: ref(null),
      inSameOrganizations: computed(() => editor.currentProjectId == null),
      active: ref(null),
      present: ref(true),
    },
    {
      live: true,
    }
  );

  return {
    ...clients,
  };
}

export const useCurrentClients = createSharedComposable(_useCurrentClients);
