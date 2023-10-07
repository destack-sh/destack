<script lang="ts" setup>
import ConfirmPopover from "@/components/basic/ConfirmPopover.vue";
import MembershipLevelSelect from "@/components/basic/MembershipLevelSelect.vue";
import Switch from "@/components/basic/Switch.vue";
import UserCombobox from "@/components/basic/UserCombobox.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql } from "@/gql";
import { OrganizationRole, type OrganizationInvite, type OrganizationMembership } from "@/gql/graphql";
import { useAuth } from "@/state/auth";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { PopoverButton } from "@headlessui/vue";
import { MinusCircleIcon, PlusIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref, type Ref } from "vue";
const props = defineProps<{ slug: string }>();

const { result: membersResult } = useQuery(
  graphql(/* GraphQL */ `
    query organizationMembers($slug: String!) {
      ownerBySlug(slug: $slug) {
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
                email
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
const organization = computed(() =>
  membersResult.value?.ownerBySlug?.__typename == "Organization" ? membersResult.value?.ownerBySlug : null
);
const memberships = computed(() => organization.value?.memberships.edges.map((e) => e.node));
const invites = computed(() => organization.value?.invites.edges.map((e) => e.node));
const canWrite = computed(() => organization.value?.canWrite ?? false);

const showInvites = ref(true);
const { getTimeFromNowLongString } = useTimeFromNow();

const addMemberRef = ref<HTMLButtonElement | null>(null);

const invitingUser: Ref<{ id?: string; email: string } | null> = ref(null);
const invitingLevel: Ref<OrganizationRole> = ref(OrganizationRole.Member);

const ops = useOperations();
const notifications = useNotifications();

async function createInvites() {
  // right now this is a primitive menuwith only one user to select
  if (invitingUser.value == null) {
    return;
  }
  // we also don't do any validation on which user can be added yet

  const organizationId = organization.value?.id;
  const ret = await ops.organization.createInvites(organizationId, [invitingUser.value?.email], invitingLevel.value);
  if (ret?.data?.createOrganizationInvites?.__typename == "Organization") {
    // success
    notifications.show({
      type: "organization.createdInvite",
      kind: "success",
      message: "Invite sent",
      description: `${invitingUser.value.email} may now join ${props.slug}.`,
    });

    // do the next one (select everything)
    invitingUser.value = null;
    addMemberRef.value?.focus();
  }
}

async function cancelInvite(invite: OrganizationInvite) {
  // not implemented
  const ret = await ops.organization.cancelInvite(invite.id);
  if (ret?.data?.cancelOrganizationInvite?.__typename == "Organization") {
    // success
    notifications.show({
      type: "organization.cancelledInvite",
      kind: "success",
      message: "Invite cancelled",
      description: `That invite has been silently disappeared.`,
    });
  }
}

async function removeMembership(membership: OrganizationMembership) {
  // not implemented
  console.log("remove membership not implemented yet", membership);
}
</script>
<template>
  <div class="h-full w-full">
    <div class="flex flex-row items-baseline justify-between">
      <p class="text-gray-900">Collaborate on Benches to build AI together.</p>
      <span class="text-gray-500">
        Show invites
        <Switch class="ml-1" v-model="showInvites" />
      </span>
    </div>
    <table
      class="mt-3 min-w-full divide-y divide-orange-900 divide-opacity-[12%] rounded-sm border border-orange-900/[12%] bg-white text-sm"
      v-show="membersResult?.ownerBySlug != null"
    >
      <thead>
        <tr>
          <th scope="col" class="px-3 py-2 text-left text-sm font-semibold text-gray-900">User</th>
          <th scope="col" class="px-3 py-2 text-left text-sm font-semibold text-gray-900">Status</th>
          <th scope="col" class="px-3 py-2 text-left text-sm font-semibold text-gray-900">Role</th>
          <th scope="col" class="px-3 py-2 text-center text-sm font-semibold text-gray-900" v-if="canWrite">
            <button
              class="mt-1 text-center text-orange-600 hover:bg-orange-100 focus:bg-gray-100 focus:outline-none"
              @click="addMemberRef?.focus"
            >
              <PlusIcon class="h-5 w-5" />
            </button>
          </th>
        </tr>
      </thead>
      <tbody>
        <!-- Members -->
        <tr v-for="membership in memberships" :key="membership.id" class="group">
          <!-- User -->
          <td class="whitespace-nowrap px-3 py-3">
            <div class="flex flex-col">
              <span
                class="flex flex-row items-baseline gap-1 text-gray-900"
                :class="membership.user.id == auth.me.value?.id ? 'text-orange-600' : ''"
              >
                <router-link :to="`/${membership.user.username}`" class="underline-offset-4 hover:underline">
                  {{ membership.user.username }}
                </router-link>
                <span v-if="membership.user.id == auth.me.value?.id"> (you) </span>
              </span>
              <span class="text-xs text-gray-500">{{ membership.user.email }}</span>
            </div>
          </td>
          <!-- Status -->
          <td class="px-3 py-3">
            <div class="flex flex-col">
              <span class="w-fit rounded-sm bg-orange-100 px-1 py-0.5 text-xs text-orange-900">Active</span>
              <span class="text-xs text-gray-500">joined {{ getTimeFromNowLongString(membership.createdAt) }}</span>
            </div>
          </td>
          <!-- Role -->
          <td class="px-3 py-3">
            <span class="text-gray-900">{{ membership.level }}</span>
          </td>
          <!-- Action -->
          <td class="px-3 py-3 text-center text-gray-400 group-hover:text-gray-700" v-if="canWrite">
            <ConfirmPopover
              v-if="auth.me.value?.id != membership.user.id"
              @action="removeMembership(membership as any)"
              title="Remove member"
              :description="`You are about to remove ${membership.user.username} from ${props.slug}.`"
              confirmText="Remove member"
              cancelText="Keep"
              v-slot="{ open }"
            >
              <PopoverButton class="hover:text-red-600" :class="open ? 'bg-orange-100 text-red-600' : ''">
                <MinusCircleIcon class="h-4 w-4" />
              </PopoverButton>
            </ConfirmPopover>
          </td>
        </tr>
        <!-- Invites (if shown) -->
        <tr v-for="invite in showInvites ? invites : []" :key="invite.id" class="group">
          <!-- User -->
          <td class="whitespace-nowrap px-3 py-3">
            <div class="flex flex-col">
              <span class="text-gray-900">
                {{ invite.user?.username ?? "(Not signed up)" }}
              </span>
              <span class="text-xs text-gray-500">{{ invite.email }}</span>
            </div>
          </td>
          <!-- Status -->
          <td class="px-3 py-3">
            <div class="flex flex-col">
              <span class="w-fit rounded-sm bg-yellow-100 px-1 py-0.5 text-xs text-yellow-900">Invited</span>
              <span class="text-xs text-gray-500">invited {{ getTimeFromNowLongString(invite.createdAt) }}</span>
            </div>
          </td>
          <!-- Role -->
          <td class="px-3 py-3">
            <span class="text-gray-900">{{ invite.level }}</span>
          </td>
          <!-- Action -->
          <td class="px-3 py-3 text-center" v-if="canWrite">
            <ConfirmPopover
              @action="cancelInvite(invite as any)"
              title="Cancel invite"
              :description="`You are about to cancel the invite to ${invite.email}.`"
              confirmText="Cancel invite"
              cancelText="Keep"
              v-slot="{ open }"
            >
              <PopoverButton class="hover:text-red-600" :class="open ? 'bg-orange-100 text-red-600' : ''">
                <MinusCircleIcon class="h-4 w-4" />
              </PopoverButton>
            </ConfirmPopover>
          </td>
        </tr>
        <!-- Create invite -->
        <tr class="border-t border-orange-900/[12%]">
          <th colspan="5" scope="colgroup" class="px-3 pb-0 pt-3 text-left text-gray-900">Grow the team</th>
        </tr>
        <tr v-if="canWrite">
          <!-- User -->
          <td class="whitespace-nowrap px-3 py-3">
            <UserCombobox ref="addMemberRef" v-model="invitingUser" placeholder="New member email" />
          </td>
          <!-- Status -->
          <td class="px-3 py-3">
            <div class="flex flex-col" v-if="invitingUser != null">
              <span class="w-fit rounded-sm bg-yellow-100 px-1 py-0.5 text-xs text-yellow-900">Excited</span>
              <span class="text-xs text-gray-500">joining soon</span>
            </div>
          </td>
          <!-- Role -->
          <td class="px-3 py-3">
            <MembershipLevelSelect v-if="invitingUser != null" v-model="invitingLevel" />
          </td>
          <!-- Action -->
          <td class="px-1 py-3 text-center">
            <button
              v-if="invitingUser"
              class="focus rounded-sm border border-orange-600 bg-white px-3 py-1 text-gray-900 hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
              @click="createInvites"
            >
              Invite
            </button>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
