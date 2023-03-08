<script lang="ts" setup>
import ConfirmPopover from "@/components/basic/ConfirmPopover.vue";
import Switch from "@/components/basic/Switch.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql } from "@/gql";
import { AccessTokenScope, AccessTokenStatus } from "@/gql/graphql";
import { useNotifications } from "@/state/notifications";
import { useOperationsStore } from "@/state/operations";
import { PopoverButton } from "@headlessui/vue";
import { PlusIcon, TrashIcon } from "@heroicons/vue/24/outline";
import { useMutation, useQuery } from "@vue/apollo-composable";
import { useClipboard } from "@vueuse/core";
import { computed, ref } from "vue";

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

const operations = useOperationsStore();
const notifications = useNotifications();
const clipboard = useClipboard();

async function createAccessToken() {
  const result = await operations.perform({
    type: "auth.createAccessToken",
    do: async () => {
      return await createAccessTokenMut({
        ownerId: accessTokensResult.value?.ownerBySlug?.id,
        scopes: [AccessTokenScope.Run],
      });
    },
  });
  if (result?.data?.createAccessToken?.__typename == "AccessTokenCreatePayload") {
    clipboard.copy(result.data.createAccessToken.token);
    notifications.show({
      type: "auth.createAccessToken",
      kind: "success",
      message: "New access token copied",
      description: "Your new access token is in your clipboard. This is your one and only chance to save it somewhere.",
      showTimeMs: 20000,
    });
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
  const result = await operations.perform({
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
const { getTimeFromNowLongString } = useTimeFromNow();
</script>
<template>
  <div class="h-full w-full">
    <div class="flex flex-row items-baseline justify-between">
      <p class="text-gray-900">Access tokens let you connect to Bench from other applications.</p>
      <span class="text-gray-500">
        Show inactive
        <Switch class="ml-1" v-model="showInactive" />
      </span>
    </div>

    <table
      class="mt-3 min-w-full divide-y divide-orange-900 divide-opacity-[12%] rounded-sm border border-orange-900 border-opacity-[12%] bg-white"
    >
      <thead>
        <tr>
          <th scope="col" class="py-2 px-3 text-left text-sm font-semibold text-gray-900">Secret key</th>
          <th scope="col" class="py-2 px-3 text-left text-sm font-semibold text-gray-900">Status</th>
          <th scope="col" class="py-2 px-3 text-left text-sm font-semibold text-gray-900">Scopes</th>
          <th scope="col" class="py-2 px-3 text-left text-sm font-semibold text-gray-900">Created</th>
          <th scope="col" class="py-2 pr-1 text-left text-sm font-semibold text-gray-900">
            <button
              class="focuus:bg-gray-100 mt-1 text-orange-600 hover:bg-orange-50 focus:outline-none"
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
          <td class="whitespace-nowrap px-3 py-3 font-mono text-gray-700">
            <div class="flex flex-col">
              <span>x-...{{ token.tokenKey }}</span>
            </div>
          </td>
          <td class="px-3">
            <span v-if="token.revokedAt != null" class="rounded-sm bg-red-50 p-1 text-red-900">revoked</span>
            <span v-else-if="token.expiresAt == null" class="rounded-sm bg-green-50 p-1 text-green-900">active</span>
            <span v-else>
              {{ getTimeFromNowLongString(token.expiresAt) }}
            </span>
          </td>
          <td class="px-3">
            <span v-for="scope in token.scopes" :key="scope" class="rounded-sm bg-orange-50 p-1 text-orange-900">
              {{ scope.toLowerCase() }}
            </span>
          </td>
          <td class="px-3 text-gray-900">{{ getTimeFromNowLongString(token.createdAt) }}</td>
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
      </tbody>
    </table>
  </div>
</template>
