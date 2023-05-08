<script lang="ts" setup>
import ConfirmPopover from "@/components/basic/ConfirmPopover.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import Switch from "@/components/basic/Switch.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { DeploymentStatus, DeploymentType, StatementType, SymbolType } from "@/gql/graphql";
import { provideGlobalAction } from "@/state/actions";
import { SYMBOL_TYPE_KEYWORD, useEditorState } from "@/state/editor";
import { ProjectHeaderType } from "@/state/fragments";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { fileOf, symbolsLike, useCurrentInterpModule } from "@/state/runtime";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { CheckIcon } from "@heroicons/vue/20/solid";
import { CloudIcon, DocumentDuplicateIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useClipboard } from "@vueuse/core";
import { computed, ref } from "vue";

const props = defineProps<{ project: FragmentType<typeof ProjectHeaderType> }>();
const emit = defineEmits<{ (e: "show"): void }>();

const project = computed(() => useFragment(ProjectHeaderType, props.project));
const editor = useEditorState();

const { result: deploymentsResult } = useQuery(
  graphql(/* GraphQL */ `
    query deployments($projectVersionId: GlobalID!) {
      projectVersion(id: $projectVersionId) {
        id
        committed
        tag
        deployments(filters: { isOwned: true }) {
          totalCount
          edges {
            node {
              id
              createdAt
              updatedAt
              type
              status
              deployAllStatements
            }
          }
        }
      }
    }
  `),
  () => ({
    projectVersionId: editor.currentProjectVersionId,
  })
);
const committed = computed(() => deploymentsResult.value?.projectVersion?.committed);
const tag = computed(() => deploymentsResult.value?.projectVersion?.tag);
const deployments = computed(() => deploymentsResult.value?.projectVersion?.deployments.edges.map((x) => x.node) || []);
const isDeployed = computed(() => deployments.value.find((d) => d.type == DeploymentType.Manual));

const ops = useOperations();
const notifications = useNotifications();

const runtime = useCurrentInterpModule();
const canDeploy = computed(
  () =>
    !isDeployed.value && project.value?.canWrite && runtime.errors?.value != null && runtime.errors.value.length == 0
);
const deploy = provideGlobalAction({
  id: "version.deployInstant",
  label: "Deploy",
  shortcuts: [],
  enabled: canDeploy,
  apply: async () => {
    await Promise.all(
      deployments.value.map(async (deployment) => {
        ops.deployment.update(deployment.id, DeploymentStatus.Active);
      })
    );
    notifications.show({
      kind: "success",
      type: "version.deploy",
      message: "Deployed",
      description: `${deployedEndpoints.value?.length} endpoints deployed.`,
      actionText: "Integrate",
      action: () => {
        // re-direct to integration page (same as "via REST" link)
      },
    });
  },
});

function archiveDeployment() {
  console.log("archive not implemented");
}

const endpoints = symbolsLike({
  types: [StatementType.Definition],
  symbolTypes: [SymbolType.Task, SymbolType.Code, SymbolType.Runconfig],
});
const deployedEndpoints = computed(() => endpoints.value); // not configurable yet

const deployButtonRef = ref<InstanceType<typeof PopoverButton> | null>(null);
provideGlobalAction({
  id: "version.deploy",
  label: "Deploy...",
  shortcuts: ["ctrl+alt+k"],
  enabled: canDeploy,
  apply: () => {
    // just open snapshot history view (containing header must be visible for popover to render)
    emit("show");
    deployButtonRef.value?.$el?.click();
  },
});

const clipboard = useClipboard();
function copyApiUrlToClipboard() {
  const url = `https://api.symbolx.com/${project.value.owner.slug}/${project.value.slug}/run`;
  clipboard.copy(url);
}
</script>

<template>
  <Popover v-slot="{ open, close }" class="relative">
    <PopoverButton
      ref="deployButtonRef"
      class="relative rounded-sm p-1 text-sm focus:outline-none"
      :class="{
        'text-gray-500 hover:bg-orange-100': !canDeploy,
        'text-orange-600 hover:bg-orange-100': canDeploy,
        'bg-orange-100': open,
      }"
    >
      <CloudIcon v-if="!isDeployed" class="h-5 w-5" />
      <template v-else>
        <!-- Already deployed (yes this is ugly :c) -->
        <CloudIcon class="h-5 w-5" />
        <CheckIcon class="absolute left-2 top-[9px] h-3 w-3" />
      </template>
    </PopoverButton>

    <FadeTransition>
      <PopoverPanel
        class="absolute right-0 top-10 z-10 mt-0 flex w-96 flex-col gap-2 rounded-sm bg-white px-4 pb-4 pt-2 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <!-- Header -->
        <div class="">
          <h2 class="font-bold text-gray-900">Manage deployment</h2>
          <p class="pt-2 text-gray-900">Deployments own the resources to run a Bench version.</p>
          <p v-if="!committed" class="text-gray-900">
            The latest version is always deployed (tagged
            <span class="rounded-sm bg-gray-200 px-0.5 font-mono">x</span>).
          </p>
          <p class="pt-0 text-gray-900">
            Deployed endpoints are available via
            <router-link
              to="/symbolx/docs#Deploying"
              target="_blank"
              class="underline decoration-gray-500 decoration-dashed underline-offset-4 hover:decoration-solid"
              >REST</router-link
            >
            at:
          </p>
          <p class="relative mt-2 w-full rounded-sm border border-orange-900 border-opacity-[20%] p-1">
            <span :href="`https://api.symbolx.com/${project.owner.slug}/${project.slug}/run`" class="text-gray-900">
              api.symbolx.com/<span class="text-orange-600">{{ project.owner.slug }}</span
              >/<span class="text-orange-600">{{ project.slug }}</span
              >/run
            </span>
            <button
              class="absolute right-1 top-[4px] rounded-sm p-0.5 text-gray-500 hover:bg-orange-100 hover:text-gray-900"
              @click="copyApiUrlToClipboard"
            >
              <DocumentDuplicateIcon class="h-4 w-4 text-gray-500" />
            </button>
          </p>
        </div>

        <!-- Endpoints -->
        <div class="mt-4">
          <div class="flex w-full flex-row justify-between">
            <h3 class="font-bold text-gray-900">
              Endpoints <span class="rounded-3xl bg-gray-200 px-1.5 font-normal">{{ deployedEndpoints.length }}</span>
            </h3>
            <!-- Deploy all? -->
            <div class="flex flex-row items-center gap-1">
              <span class="text-gray-500">All</span>
              <Switch :model-value="true" />
            </div>
          </div>
          <!-- Deploy specific endpoints -->
          <ul class="mt-2 flex flex-col">
            <li v-for="endpoint in endpoints" :key="endpoint.id" class="flex flex-row items-center gap-4">
              <!-- Endpoint info -->
              <div class="flex flex-1 items-baseline justify-between gap-1">
                <h3>
                  {{ SYMBOL_TYPE_KEYWORD[endpoint.symbolType as SymbolType] }}
                  {{ endpoint.name }}
                </h3>
                <span class="text-gray-500">
                  {{ fileOf(endpoint)?.path }}
                </span>
              </div>
              <!-- Select for deployment -->
              <Switch :model-value="true" />
            </li>
          </ul>
        </div>

        <!-- Deploy action/notice -->
        <div class="mt-4 text-right">
          <!-- Actions (only if not working at head) -->
          <div class="flex flex-row justify-between" v-if="committed">
            <!-- Archive/kill if live -->
            <ConfirmPopover
              v-if="isDeployed"
              :disabled="!committed"
              title="Archive deployment"
              :description="`Archiving will shut deployment ${tag} down soon. No data lost.`"
              confirm-text="Archive"
              cancel-text="Keep"
              @action="archiveDeployment()"
              v-slot="{ open }"
            >
              <PopoverButton
                class="w-fit self-end border border-transparent px-3 py-1 text-gray-700 hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
                :class="{ 'pointer-events-none opacity-50': !canDeploy, 'bg-orange-100': open }"
              >
                Archive deployment
              </PopoverButton>
            </ConfirmPopover>
            <span v-else class="px-3 py-1 font-bold hover:cursor-not-allowed">{{ tag }} is not live</span>
            <!-- Deploy if not live -->
            <button
              v-if="!isDeployed"
              class="w-fit self-end border border-orange-600 px-3 py-1 hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
              :class="{ 'pointer-events-none opacity-50': !canDeploy }"
              @click="
                deploy.apply();
                close();
              "
            >
              Deploy
            </button>
            <span v-else class="px-3 py-1 font-bold hover:cursor-not-allowed">{{ tag }} is live</span>
          </div>
          <!-- Notices -->
          <p v-if="!project.canWrite" class="pt-1 text-xs text-yellow-600">You cannot deploy other's Benches yet.</p>
          <p v-else-if="!canDeploy && !isDeployed" class="pt-1 text-xs text-red-600">
            There are errors. Fix them to deploy.
          </p>
          <p v-else-if="endpoints.length == 0" class="text-yellow-600">There's nothing to deploy, but you could.</p>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
