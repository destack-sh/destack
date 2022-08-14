<template>
  <div ref="container" class="w-full divide-y divide-gray-200 overflow-hidden rounded-lg bg-white shadow">
    <div class="flex flex-row items-center justify-between py-2 px-4">
      <div class="flex flex-row items-center gap-2">
        <span class="ring-3 inline-flex rounded-lg bg-orange-50 p-2 text-orange-700 ring-white">
          <component :is="icon" class="h-6 w-6" aria-hidden="true" />
        </span>
        <div>
          <h3 class="text-md truncate font-medium leading-6 text-gray-900">
            {{ metaStore.functionHandlersById[node.function_id].name }}
          </h3>
          <h5 class="text-xs font-normal text-gray-500">{{ node.name }}</h5>
        </div>
      </div>
      <Menu v-if="editable" as="div" class="inline-block text-left">
        <div>
          <MenuButton
            class="flex items-center rounded-full text-gray-400 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2 focus:ring-offset-gray-100"
          >
            <span class="sr-only">Open options</span>
            <DotsVerticalIcon class="h-5 w-5" aria-hidden="true" />
          </MenuButton>
        </div>

        <transition
          enter-active-class="transition ease-out duration-100"
          enter-from-class="transform opacity-0 scale-95"
          enter-to-class="transform opacity-100 scale-100"
          leave-active-class="transition ease-in duration-75"
          leave-from-class="transform opacity-100 scale-100"
          leave-to-class="transform opacity-0 scale-95"
        >
          <MenuItems
            class="absolute right-0 z-20 mt-2 w-56 origin-top-right divide-y divide-gray-100 rounded-md bg-white py-1 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none"
          >
            <MenuItem v-slot="{ active }" v-for="action in actions" :key="action.key">
              <a
                href="#"
                @click="action.action"
                :class="[
                  active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
                  'group flex items-center px-4 py-2 text-sm',
                ]"
              >
                <component
                  :is="action.icon"
                  class="mr-3 h-5 w-5 text-gray-400 group-hover:text-gray-500"
                  aria-hidden="true"
                />
                {{ action.label }}
              </a>
            </MenuItem>
          </MenuItems>
        </transition>
      </Menu>
    </div>
    <div class="px-6 py-2 text-sm">
      <ConfigArtifactForm :model-value="configArtifacts" :spec="Object.values(configSpec).filter(isArtifactSpec)" />
    </div>
  </div>
</template>
<script lang="ts" setup>
import { useMetaStore } from "@/stores";
import type { FlowNode, FlowVersion } from "@/types/flows";
import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/vue";
import { DotsVerticalIcon, DuplicateIcon, PencilAltIcon, TrashIcon } from "@heroicons/vue/solid";
import { computed, ref } from "vue";
import ConfigArtifactForm from "@/components/ConfigArtifactForm.vue";
import { artifactConnections } from "@/utils/flows";
import { isArtifactSpec } from "@/types/spec";
import { useIcons } from "@/composables/useIcons";
import { useElementSize } from "@vueuse/core";

const props = defineProps<{
  flow: FlowVersion;
  node: FlowNode;
  editable?: boolean;
}>();
const emit = defineEmits<{ (e: "edit"): void; (e: "delete"): void }>();
const actions = [
  {
    action: () => emit("edit"),
    key: "edit",
    icon: PencilAltIcon,
    label: "Edit",
  },
  {
    action: () => emit("delete"),
    key: "delete",
    icon: TrashIcon,
    label: "Delete",
  },
  {
    action: () => ({}),
    key: "Duplicate",
    icon: DuplicateIcon,
    label: "Duplicate",
  },
];

const metaStore = useMetaStore();
const icons = useIcons();

const configArtifacts = computed(() => artifactConnections(props.flow, props.node, "argument"));
const configSpec = computed(() => metaStore.functionHandlersById[props.node.function_id].config_spec);
const icon = computed(() => icons.forNode(props.node));

const container = ref(null);
const elementSize = useElementSize(container);

defineExpose({ elementSize });
</script>
