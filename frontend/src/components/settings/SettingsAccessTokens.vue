<script lang="ts" setup>
import ConfirmPopover from "@/components/basic/ConfirmPopover.vue";
import Switch from "@/components/basic/Switch.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql } from "@/gql";
import { AccessTokenScope, AccessTokenStatus } from "@/gql/graphql";
import { useNotifications } from "@/state/notifications";
import { useOperationsStore } from "@/state/operations";
import { PopoverButton } from "@headlessui/vue";
import { DocumentDuplicateIcon, EyeIcon, EyeSlashIcon, PlusIcon, TrashIcon } from "@heroicons/vue/24/outline";
import { useApolloClient, useMutation, useQuery } from "@vue/apollo-composable";
import { useClipboard } from "@vueuse/core";
import { computed, ref, type Ref } from "vue";

const props = defineProps<{ slug: string }>();

const showInactive = ref(false);

const { result: accessTokensResult } = useQuery(
  graphql(/* GraphQL */ `
    query profileAccessTokens($slug: String!, $includeInactive: Boolean!) {
      ownerBySlug(slug: $slug) {
        ... on User {
          id
          accessTokens(filters: { includeInactive: $includeInactive }) {
            totalCount
            edges {
              node {
                id
                name
                tokenKey
                createdAt
                updatedAt
                expiresAt
                revokedAt
                status
                scopes
              }
            }
          }
        }
        ... on Organization {
          id
          accessTokens(filters: { includeInactive: $includeInactive }) {
            totalCount
            edges {
              node {
                id
                name
                tokenKey
                createdAt
                updatedAt
                expiresAt
                revokedAt
                status
                scopes
              }
            }
          }
        }
      }
    }
  `),
  computed(() => ({
    slug: props.slug,
    includeInactive: showInactive.value,
  }))
);

const { mutate: createAccessTokenMut, loading: creating } = useMutation(
  graphql(/* GraphQL */ `
    mutation createAccessToken(
      $ownerId: GlobalID!
      $scopes: [AccessTokenScope!]!
      $expiresAt: DateTime
      $name: String
    ) {
      createAccessToken(input: { ownerId: $ownerId, scopes: $scopes, expiresAt: $expiresAt, name: $name }) {
        ... on AccessTokenCreatePayload {
          token
          accessToken {
            id
            name
            tokenKey
            createdAt
            updatedAt
            expiresAt
            revokedAt
            status
            scopes
          }
        }
        ...OperationInfoContent
      }
    }
  `),
  {
    refetchQueries: ["profileAccessTokens"],
  }
);

const ops = useOperationsStore();
const notifications = useNotifications();
const clipboard = useClipboard();
const client = useApolloClient();
const { getTimeFromNowLongString } = useTimeFromNow();
const revealedAccessTokens: Ref<Record<string, string>> = ref({});

function toggleRevealed(token: { id: string }) {
  if (revealedAccessTokens.value[token.id] == null) {
    revealAccessToken(token);
  } else {
    delete revealedAccessTokens.value[token.id];
  }
}

async function copy(token: { id: string }) {
  if (revealedAccessTokens.value[token.id] == null) {
    await revealAccessToken(token);
  }
  clipboard.copy(revealedAccessTokens.value[token.id]);
  notifications.show({
    type: "auth.copyAccessToken",
    kind: "success",
    message: "Access token copied",
    description: `You can always view it again.`,
    showTimeMs: 7000,
  });
}

async function createAccessToken() {
  const result = await ops.perform({
    type: "auth.createAccessToken",
    do: async () => {
      return await createAccessTokenMut({
        ownerId: accessTokensResult.value?.ownerBySlug?.id,
        scopes: [AccessTokenScope.Run],
      });
    },
  });
  if (result?.data?.createAccessToken?.__typename == "AccessTokenCreatePayload") {
    const created = result.data.createAccessToken;
    clipboard.copy(created.token);
    notifications.show({
      type: "auth.createAccessToken",
      kind: "success",
      message: "New access token copied",
      description: "You can always view it again.",
      showTimeMs: 20000,
    });
    revealedAccessTokens.value[created.accessToken.id] = created.token;
  }
}

const { mutate: revokeAccessTokenMut } = useMutation(
  graphql(/* GraphQL */ `
    mutation revokeAccessToken($id: GlobalID!) {
      revokeAccessToken(id: $id) {
        ... on AccessToken {
          id
          revokedAt
          status
        }
        ...OperationInfoContent
      }
    }
  `),
  {
    refetchQueries: ["profileAccessTokens"],
  }
);

async function revokeAccessToken(token: { id: string }) {
  const result = await ops.perform({
    type: "auth.revokeAccessToken",
    do: async () => {
      return await revokeAccessTokenMut({ id: token.id });
    },
  });
  if (result?.data?.revokeAccessToken?.__typename == "AccessToken") {
    notifications.show({
      type: "auth.revokeAccessToken",
      kind: "success",
      message: "Access token revoked",
      description: "Access token has been revoked.",
    });
  }
}

async function revealAccessToken(token: { id: string }) {
  const ret = await client.client.query({
    query: graphql(/* GraphQL */ `
      query revealAccessToken($id: GlobalID!) {
        accessToken(id: $id) {
          ... on AccessToken {
            id
            valueRevealed
          }
        }
      }
    `),
    variables: { id: token.id },
    fetchPolicy: "no-cache",
  });
  if (ret.data.accessToken?.valueRevealed != null) {
    revealedAccessTokens.value[token.id] = ret.data.accessToken.valueRevealed;
  }
}
</script>
<template>
  <div class="h-full w-full">
    <div class="flex flex-row items-baseline justify-between">
      <p class="text-gray-900">Access tokens let you connect to Bench with code.</p>
      <span class="text-gray-500">
        Show inactive
        <Switch class="ml-1" v-model="showInactive" />
      </span>
    </div>

    <table
      class="mt-3 min-w-full divide-y divide-orange-900 divide-opacity-[12%] rounded-sm border border-orange-900/[12%] bg-white"
    >
      <thead>
        <tr>
          <th scope="col" class="px-3 py-2 text-left text-sm font-semibold text-gray-900">Secret key</th>
          <th scope="col" class="px-3 py-2 text-left text-sm font-semibold text-gray-900">Status</th>
          <th scope="col" class="px-3 py-2 text-left text-sm font-semibold text-gray-900">Created</th>
          <th scope="col" class="py-2 pr-1 text-left text-sm font-semibold text-gray-900">
            <button
              class="mt-1 text-orange-600 hover:bg-orange-100 focus:bg-gray-100 focus:outline-none"
              @click="createAccessToken"
              :disabled="creating"
            >
              <PlusIcon class="h-5 w-5" />
            </button>
          </th>
        </tr>
      </thead>
      <tbody class="divide-y divide-orange-900 divide-opacity-[12%]">
        <tr
          v-for="token in accessTokensResult?.ownerBySlug?.accessTokens?.edges.map((e) => e.node)"
          :key="token.id"
          class="group text-sm"
        >
          <!-- Token -->
          <td class="w-1/2 whitespace-nowrap px-3 py-3 font-mono text-gray-700">
            <div class="flex w-full flex-row rounded-sm bg-gray-100 px-0.5">
              <span class="">
                <template v-if="revealedAccessTokens[token.id] == null"> x-...{{ token.tokenKey }}</template>
                <template v-else>{{ revealedAccessTokens[token.id] }}</template>
              </span>
              <!-- Reveal/copy -->
              <div class="ml-auto">
                <button class="p-0.5 text-gray-400 hover:bg-orange-100" @click="toggleRevealed(token)">
                  <component :is="revealedAccessTokens[token.id] == null ? EyeSlashIcon : EyeIcon" class="h-4 w-4" />
                </button>
                <button class="ml-0.5 p-0.5 text-gray-400 hover:bg-orange-100" @click="copy(token)">
                  <DocumentDuplicateIcon class="h-4 w-4" />
                </button>
              </div>
            </div>
          </td>
          <!-- Status -->
          <td class="px-3">
            <span v-if="token.revokedAt != null" class="rounded-sm bg-red-50 p-1 text-red-900">revoked</span>
            <span v-else-if="token.expiresAt == null" class="rounded-sm bg-green-50 p-1 text-green-900">active</span>
            <span v-else>
              {{ getTimeFromNowLongString(token.expiresAt) }}
            </span>
          </td>
          <td class="px-3 text-gray-900">{{ getTimeFromNowLongString(token.createdAt) }}</td>
          <!-- Action (revoke) -->
          <td>
            <ConfirmPopover
              title="Revoke token"
              :description="`Revoking token ${token.tokenKey} will deny any future access.`"
              confirm-text="Revoke token"
              cancel-text="Keep"
              @action="revokeAccessToken(token)"
            >
              <PopoverButton
                v-if="token.status == AccessTokenStatus.Active"
                class="text-gray-300 group-hover:text-gray-500"
              >
                <TrashIcon class="h-4 w-4 hover:text-red-600" />
              </PopoverButton>
            </ConfirmPopover>
          </td>
        </tr>
        <!-- Empty state -->
        <tr v-if="accessTokensResult?.ownerBySlug?.accessTokens?.edges.length == 0">
          <td colspan="4" class="px-3 py-3 text-sm text-gray-400">No access tokens</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
