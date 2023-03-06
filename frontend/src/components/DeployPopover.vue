<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { DeploymentStatus, StatementType, SymbolType } from "@/gql/graphql";
import { provideGlobalAction, useActions } from "@/state/actions";
import { SYMBOL_TYPE_KEYWORD, useEditorState } from "@/state/editor";
import { ProjectHeaderType } from "@/state/fragments";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { symbolsLike, useCurrentModuleRuntime, fileOf } from "@/state/runtime";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { CloudArrowUpIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { computed } from "vue";

const props = defineProps<{ project: FragmentType<typeof ProjectHeaderType> }>();
const project = computed(() => useFragment(ProjectHeaderType, props.project));
const editor = useEditorState();

const { result: deploymentsResult } = useQuery(
  graphql(/* GraphQL */ `
    query projectDeployments($projectVersionId: GlobalID!) {
      projectVersion(id: $projectVersionId) {
        id
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
const deployments = computed(() => deploymentsResult.value?.projectVersion?.deployments.edges.map((x) => x.node) || []);

const actions = useActions();
const operations = useOperations();
const notifications = useNotifications();

const runtime = useCurrentModuleRuntime();
const canDeploy = computed(
  () => project.value.canWrite && runtime.errors?.value != null && runtime.errors.value.length == 0
);
const deploy = provideGlobalAction({
  id: "version.deploy",
  label: "Deploy",
  shortcuts: [],
  apply: async () => {
    // re-use random name/tagging logic from action for now, will be done inline here later
    await actions.apply("version.commit");
    await Promise.all(
      deployments.value.map(async (deployment) => {
        operations.deployment.update(deployment.id, DeploymentStatus.Active);
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

const endpoints = symbolsLike({
  types: [StatementType.Definition],
  symbolTypes: [SymbolType.Task, SymbolType.Runconfig],
});
const deployedEndpoints = computed(() => endpoints.value); // not configurable yet
</script>

<template>
  <Popover v-slot="{ open }" class="relative">
    <PopoverButton
      class="rounded-sm p-1 text-sm focus:outline-none"
      :class="{
        'text-gray-500 hover:bg-orange-50': !canDeploy,
        'text-orange-600 hover:bg-orange-50': canDeploy,
        'bg-orange-50': open,
      }"
    >
      <CloudArrowUpIcon class="h-5 w-5" />
    </PopoverButton>

    <FadeTransition>
      <PopoverPanel
        class="absolute top-10 right-0 z-10 mt-0 flex w-96 flex-col gap-2 rounded-sm bg-white px-4 pt-2 pb-4 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <!-- Header -->
        <div class="">
          <h2 class="font-bold text-gray-900">Deployment</h2>
          <p class="pt-2 text-gray-900">
            Access
            <router-link
              to="/symbolx/docs#Deploying"
              target="_blank"
              class="underline decoration-gray-500 decoration-dashed underline-offset-4 hover:decoration-solid"
              >deployed</router-link
            >
            endpoints
            <button
              target="_blank"
              class="underline decoration-gray-500 decoration-dashed underline-offset-4 hover:decoration-solid"
            >
              via REST
            </button>
            at:
          </p>
          <p class="mt-2 w-full rounded-sm border border-gray-200 p-1">
            <a
              :href="`https://api.symbolx.com/${project.owner.slug}/${project.slug}/run`"
              class="text-gray-900 underline-offset-4 hover:underline"
            >
              api.symbolx.com/{{ project.owner.slug }}/{{ project.slug }}/run
            </a>
          </p>
          <p class="mt-1 text-xs text-gray-500">Hint: 'x' refers to the live working version.</p>
        </div>

        <!-- Endpoints -->
        <ul class="mt-4 flex flex-col">
          <li v-for="endpoint in endpoints" :key="endpoint.id" class="flex flex-row items-baseline gap-2">
            <!-- Select for deployment -->
            <div>
              <!-- Not configurable yet -->
              <input
                type="checkbox"
                checked
                disabled
                class="h-4 w-4 rounded-sm border-gray-300 text-orange-600 focus:ring-0"
              />
            </div>
            <!-- Endpoint info -->
            <div class="flex flex-1 items-baseline justify-between gap-1">
              <h3>
                {{ SYMBOL_TYPE_KEYWORD[endpoint.symbolType as SymbolType] }}
                {{ endpoint.name }}
              </h3>
              <span class="text-xs text-gray-500">
                {{ fileOf(endpoint)?.path }}
              </span>
            </div>
          </li>
        </ul>

        <!-- Snapshot name/tag/description -->
        <!-- need to be able to configure version name, tag, description here -->

        <!-- Deploy action -->
        <div class="mt-4 text-right">
          <button
            class="w-fit self-end border border-orange-600 px-3 py-1 hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
            :class="{ 'pointer-events-none opacity-50': !canDeploy }"
            @click="deploy.apply"
          >
            Deploy
          </button>
          <p v-if="!project.canWrite" class="pt-1 text-xs text-yellow-600">You cannot deploy other's Benches yet.</p>
          <p v-else-if="!canDeploy" class="pt-1 text-xs text-red-600">There are errors. Fix them to deploy.</p>
          <p v-else-if="endpoints.length == 0" class="text-yellow-600">There's nothing to deploy, but you could.</p>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
