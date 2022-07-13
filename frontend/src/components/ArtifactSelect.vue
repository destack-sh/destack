<template>
  <TransitionRoot :show="open" as="template" @after-leave="query = ''" appear>
    <Dialog as="div" class="relative z-10" @close="open = false">
      <TransitionChild
        as="template"
        enter="ease-out duration-300"
        enter-from="opacity-0"
        enter-to="opacity-100"
        leave="ease-in duration-200"
        leave-from="opacity-100"
        leave-to="opacity-0"
      >
        <div class="fixed inset-0 bg-gray-500 bg-opacity-25 transition-opacity" />
      </TransitionChild>

      <div class="fixed inset-0 z-10 overflow-y-auto p-4 sm:p-6 md:p-20">
        <TransitionChild
          as="template"
          enter="ease-out duration-300"
          enter-from="opacity-0 scale-95"
          enter-to="opacity-100 scale-100"
          leave="ease-in duration-200"
          leave-from="opacity-100 scale-100"
          leave-to="opacity-0 scale-95"
        >
          <DialogPanel
            class="mx-auto max-w-xl transform rounded-xl bg-white p-2 shadow-2xl ring-1 ring-black ring-opacity-5 transition-all"
          >
            <h3 as="h3" class="px-4 pt-1 pb-2 text-lg font-medium leading-6 text-gray-900">
              Select model
            </h3>
            <Combobox @update:modelValue="onSelect">
              <ComboboxInput
                class="w-full rounded-md border-0 bg-gray-100 px-4 py-2.5 text-gray-900 placeholder-gray-500 focus:ring-0 sm:text-sm"
                placeholder="Search..."
                @change="query = $event.target.value"
              />

              <ComboboxOptions
                v-if="filteredArtifacts.length > 0"
                static
                class="-mb-2 max-h-72 scroll-py-2 overflow-y-auto py-2 text-sm text-gray-800"
              >
                <ComboboxOption
                  v-for="artifact in filteredArtifacts"
                  :key="artifact.id"
                  :value="artifact"
                  as="template"
                  v-slot="{ active }"
                >
                  <li
                    :class="[
                      'cursor-default select-none rounded-md px-4 py-2',
                      active && 'bg-orange-600 text-white',
                    ]"
                  >
                    {{ artifact.name }}
                  </li>
                </ComboboxOption>
              </ComboboxOptions>

              <div
                v-if="query !== '' && filteredArtifacts.length === 0"
                class="py-14 px-4 text-center sm:px-14"
              >
                <ChipIcon class="mx-auto h-6 w-6 text-gray-400" aria-hidden="true" />
                <p class="mt-4 text-sm text-gray-900">No artifacts found using that search term.</p>
              </div>
            </Combobox>
          </DialogPanel>
        </TransitionChild>
      </div>
    </Dialog>
  </TransitionRoot>
</template>

<script lang="ts" setup>
import { useArtifactsStore } from "@/stores";
import type { Artifact } from "@/types";
import {
  Combobox,
  ComboboxInput,
  ComboboxOption,
  ComboboxOptions,
  Dialog,
  DialogPanel,
  TransitionChild,
  TransitionRoot,
} from "@headlessui/vue";
import { ChipIcon } from "@heroicons/vue/outline";
import { computed, ref } from "vue";

const open = ref(false);
const query = ref("");

const artifactsStore = useArtifactsStore();
const filteredArtifacts = computed(() =>
  artifactsStore.models.filter((artifact) => {
    return artifact.name.toLowerCase().includes(query.value.toLowerCase());
  })
);

const emit = defineEmits(["select"]);
function onSelect(artifact: Artifact) {
  emit("select", artifact);
  hide();
}

function show() {
  open.value = true;
}

function hide() {
  open.value = false;
}

defineExpose({ show, hide });
</script>
