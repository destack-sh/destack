<template>
  <Sidebar>
    <form class="mx-auto max-w-xl space-y-8 divide-y divide-gray-200 pt-8">
      <div class="space-y-8 divide-y divide-gray-200">
        <div>
          <div>
            <h3 class="text-lg font-medium leading-6 text-gray-900">Create a new model</h3>
            <p class="mt-1 text-sm text-gray-500">
              This information will be displayed publicly so be careful what you share.
            </p>
          </div>

          <div class="mt-6 grid grid-cols-1 gap-y-6 gap-x-4 sm:grid-cols-6">
            <div class="sm:col-span-4">
              <label for="name" class="block text-sm font-medium text-gray-700"> Model name </label>
              <div class="mt-1 flex rounded-md shadow-sm">
                <span
                  class="inline-flex items-center rounded-l-md border border-r-0 border-gray-300 bg-gray-50 px-3 text-gray-500 sm:text-sm"
                >
                  symbolx.co/models/
                </span>
                <input
                  v-model="name"
                  type="text"
                  name="name"
                  id="name"
                  autocomplete="name"
                  pattern="[A-Za-z0-9]"
                  required
                  class="block w-full min-w-0 flex-1 rounded-none rounded-r-md border-gray-300 focus:border-orange-500 focus:ring-orange-500 sm:text-sm"
                />
              </div>
            </div>

            <div class="sm:col-span-6">
              <label for="description" class="block text-sm font-medium text-gray-700">
                Description <span class="font-normal text-gray-500">(optional)</span>
              </label>
              <div class="mt-1">
                <textarea
                  v-model="description"
                  id="description"
                  name="description"
                  rows="1"
                  class="block w-full rounded-md border border-gray-300 shadow-sm focus:border-orange-500 focus:ring-orange-500 sm:text-sm"
                />
              </div>
            </div>
          </div>
        </div>
        <div>
          <Listbox as="div" v-model="selectedTemplate">
            <ListboxLabel class="mt-2 block text-sm font-medium text-gray-700">
              Model template
            </ListboxLabel>
            <div class="relative mt-1">
              <ListboxButton
                class="relative w-full cursor-default rounded-md border border-gray-300 bg-white py-2 pl-3 pr-10 text-left shadow-sm focus:border-orange-500 focus:outline-none focus:ring-1 focus:ring-orange-500 sm:text-sm"
              >
                <span
                  class="block truncate"
                  :class="selectedTemplate.empty ? 'text-gray-500' : ''"
                  >{{ selectedTemplate.name }}</span
                >
                <span class="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2">
                  <SelectorIcon class="h-5 w-5 text-gray-400" aria-hidden="true" />
                </span>
              </ListboxButton>

              <transition
                leave-active-class="transition ease-in duration-100"
                leave-from-class="opacity-100"
                leave-to-class="opacity-0"
              >
                <ListboxOptions
                  class="absolute z-10 mt-1 max-h-60 w-full overflow-auto rounded-md bg-white py-1 text-base shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none sm:text-sm"
                >
                  <ListboxOption
                    as="template"
                    v-for="template in availableTemplates"
                    :key="template.name"
                    :value="template"
                    v-slot="{ active, selected }"
                  >
                    <li
                      :class="[
                        active ? 'bg-orange-600 text-white' : 'text-gray-900',
                        'relative cursor-default select-none py-2 pl-8 pr-4',
                      ]"
                    >
                      <span :class="[selected ? 'font-semibold' : 'font-normal', 'block truncate']">
                        {{ template.name }}
                      </span>

                      <span
                        v-if="selected"
                        :class="[
                          active ? 'text-white' : 'text-orange-600',
                          'absolute inset-y-0 left-0 flex items-center pl-1.5',
                        ]"
                      >
                        <CheckIcon class="h-5 w-5" aria-hidden="true" />
                      </span>
                    </li>
                  </ListboxOption>
                </ListboxOptions>
              </transition>
            </div>
          </Listbox>
        </div>
      </div>

      <div class="pt-5">
        <div class="flex justify-end">
          <button
            type="submit"
            class="ml-3 inline-flex justify-center rounded-md border border-transparent bg-orange-600 py-2 px-4 text-sm font-medium text-white shadow-sm hover:bg-orange-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
            @click.prevent="submit"
          >
            Create
          </button>
        </div>
      </div>
    </form>
  </Sidebar>
</template>
<script lang="ts" setup>
import { api } from "@/api";
import { useArtifactsStore } from "@/stores";
import type { Artifact } from "@/types";
import {
  Listbox,
  ListboxButton,
  ListboxLabel,
  ListboxOption,
  ListboxOptions,
} from "@headlessui/vue";
import { CheckIcon, SelectorIcon } from "@heroicons/vue/solid";
import { ref, type Ref } from "vue";
import { useRouter } from "vue-router";
import Sidebar from "./Sidebar.vue";

const name: Ref<string> = ref("");
const description: Ref<string> = ref("");

type ModelTemplate = {
  name: string;
  empty?: boolean;
};

const availableTemplates = [
  { name: "No template", empty: true },
  { name: "Spacy Bundled" },
  { name: "Spacy Custom" },
  { name: "HuggingFace Hub" },
  { name: "HuggingFace Custom" },
];
const selectedTemplate: Ref<ModelTemplate> = ref(availableTemplates[0]);

const router = useRouter();
const artifactsStore = useArtifactsStore();

function submit() {
  api
    .post<Artifact>("/models", {
      name: name.value,
      description: description.value,
    })
    .then((response) => response.data)
    .then((artifact) => {
      artifactsStore.hydrate();
      router.push(`/models/${artifact.name}`);
    });
}
</script>
