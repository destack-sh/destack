<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useFragment, type FragmentType } from "@/gql";
import { StatementType, SymbolType } from "@/gql/graphql";
import { provideGlobalAction, useActions } from "@/state/actions";
import { SYMBOL_TYPE_KEYWORD, useEditorState } from "@/state/editor";
import { ProjectHeaderType } from "@/state/fragments";
import { symbolsLike, useCurrentModuleRuntime, fileOf } from "@/state/runtime";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { CloudArrowUpIcon } from "@heroicons/vue/24/outline";
import { computed } from "vue";

const props = defineProps<{ project: FragmentType<typeof ProjectHeaderType> }>();
const project = computed(() => useFragment(ProjectHeaderType, props.project));

const editor = useEditorState();
const actions = useActions();

const runtime = useCurrentModuleRuntime();
const canDeploy = computed(() => runtime.errors?.value != null && runtime.errors.value.length == 0);
const deploy = provideGlobalAction({
  id: "version.deploy",
  label: "Deploy",
  shortcuts: [],
  apply: () => {
    console.log("deploy");
  },
});

const endpoints = symbolsLike({
  types: [StatementType.Definition],
  symbolTypes: [SymbolType.Task, SymbolType.Runconfig],
});
</script>

<template>
  <Popover v-slot="{ open }" class="relative">
    <PopoverButton
      class="rounded-sm p-1 text-sm focus:outline-none"
      :class="{
        'text-gray-500': !canDeploy,
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
        <div>
          <h2 class="font-bold text-gray-900">Deployment</h2>
          <p class="pt-2 text-gray-900">Deployed endpoints are available <a>via REST</a> at:</p>
          <a
            :href="`https://api.symbolx.com/${project.owner.slug}/${project.slug}/run`"
            class="pt-0.5 text-orange-600 decoration-orange-600 underline-offset-4 hover:underline"
          >
            api.symbolx.com/{{ project.owner.slug }}/{{ project.slug }}/run
          </a>
        </div>

        <!-- Endpoints -->
        <ul class="mt-4 flex flex-col">
          <li v-for="endpoint in endpoints" :key="endpoint.id" class="flex flex-row items-baseline gap-2">
            <!-- Select for deployment -->
            <div>
              <!-- Not configurable yet -->
              <input type="checkbox" checked class="h-4 w-4 rounded-sm border-gray-300 text-orange-600" />
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
          <p v-if="!canDeploy" class="pt-1 text-xs text-red-600">There are errors. Fix them to deploy.</p>
          <p v-else-if="endpoints.length == 0" class="text-yellow-600">There's nothing to deploy, but you could.</p>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
