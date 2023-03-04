<script lang="ts" setup>
import Switch from "@/components/basic/Switch.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql } from "@/gql";
import { useAuth } from "@/state/auth";
import { useNotifications } from "@/state/notifications";
import { useOperationsStore } from "@/state/operations";
import { MinusCircleIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref } from "vue";
const props = defineProps<{ slug: string }>();

const { result: membersResult, loading } = useQuery(
  graphql(/* GraphQL */ `
    query organizationMembers($slug: String!) {
      organizationBySlug(organization: $slug) {
        ... on Organization {
          id
          canWrite
          memberships {
            totalCount
            edges {
              node {
                id
                createdAt
                level
                user {
                  id
                  slug
                  email
                  name
                  username
                }
              }
            }
          }
          invites {
            totalCount
            edges {
              node {
                id
                createdAt
                level
                emailSentAt
                user {
                  id
                  slug
                  email
                  name
                  username
                }
              }
            }
          }
        }
      }
    }
  `),
  {
    slug: props.slug,
  }
);

const auth = useAuth();
const memberships = computed(() => membersResult.value?.organizationBySlug?.memberships.edges.map((e) => e.node));
const invites = computed(() => membersResult.value?.organizationBySlug?.invites.edges.map((e) => e.node));
const canWrite = computed(() => membersResult.value?.organizationBySlug?.canWrite ?? false);

const operations = useOperationsStore();
const notifications = useNotifications();

const showInvites = ref(true);
const { getTimeFromNowLongString } = useTimeFromNow();
</script>
<template>
  <div class="h-full w-full">
    <div class="flex flex-row items-baseline justify-between">
      <p class="text-gray-900">Organization members can share Benches.</p>
      <span class="text-gray-500">
        Show invites
        <Switch class="ml-1" v-model="showInvites" />
      </span>
    </div>
    <table class="mt-3 min-w-full divide-y divide-gray-300 rounded-sm border border-gray-200 bg-white text-sm">
      <thead>
        <tr>
          <th scope="col" class="py-2 px-3 text-left text-sm font-semibold text-gray-900">User</th>
          <th scope="col" class="py-2 px-3 text-left text-sm font-semibold text-gray-900">Status</th>
          <th scope="col" class="py-2 px-3 text-left text-sm font-semibold text-gray-900">Role</th>
          <th scope="col" class="py-2 px-3 text-left text-sm font-semibold text-gray-900" v-if="canWrite">
            <span class="sr-only">Remove</span>
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="membership in memberships" :key="membership.id">
          <td class="whitespace-nowrap px-3 py-3">
            <div class="flex flex-col">
              <span class="flex flex-row items-baseline gap-1 text-gray-900">
                <router-link :to="`/${membership.user.username}`" class="underline-offset-4 hover:underline">
                  {{ membership.user.username }}
                </router-link>
                <span v-if="membership.user.id == auth.me.value?.id" class="rounded-sm px-1 text-orange-900">
                  (you)
                </span>
              </span>
              <span class="text-xs text-gray-500">{{ membership.user.email }}</span>
            </div>
          </td>
          <td class="px-3 py-3">
            <div class="flex flex-col">
              <span class="w-fit rounded-sm bg-orange-100 py-0.5 px-1 text-xs text-orange-900">Active</span>
              <span class="text-xs text-gray-500">joined {{ getTimeFromNowLongString(membership.createdAt) }}</span>
            </div>
          </td>
          <td class="px-3 py-3">
            <span class="text-gray-900">{{ membership.level }}</span>
          </td>
          <td class="px-3 py-3" v-if="canWrite">
            <button v-if="auth.me.value?.id != membership.user.id">
              <MinusCircleIcon class="h-4 w-4 text-gray-400 hover:text-gray-700" />
            </button>
          </td>
        </tr>
        <tr v-for="invite in showInvites ? invites : []" :key="invite.id">
          <td class="whitespace-nowrap px-3 py-3">
            <div class="flex flex-col">
              <span class="text-gray-900">
                {{ invite.user.username ?? "Not signed up" }}
              </span>
              <span class="text-xs text-gray-500">{{ invite.user.email }}</span>
            </div>
          </td>
          <td class="px-3 py-3">
            <div class="flex flex-col">
              <span class="w-fit rounded-sm bg-yellow-100 py-0.5 px-1 text-xs text-yellow-900">Pending</span>
              <span class="text-xs text-gray-500">invited {{ getTimeFromNowLongString(invite.createdAt) }}</span>
            </div>
          </td>
          <td class="px-3 py-3">
            <span class="text-gray-900">{{ invite.level }}</span>
          </td>
          <td class="px-3 py-3" v-if="canWrite">
            <button>
              <MinusCircleIcon class="h-4 w-4 text-gray-400 hover:text-gray-700" />
            </button>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
