<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { getRandomName } from "@/composables/useRandomName";
import type { ProjectVersion } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { bumpSemVer, FIRST_SEMVER, parseSemVer, renderSemVer, type SemVer } from "@/utils/semver";
import { Popover, PopoverPanel } from "@headlessui/vue";
import { TagIcon } from "@heroicons/vue/24/outline";
import { computed, ref, type Ref } from "vue";

const props = defineProps<{ version: ProjectVersion; prevSemVerTag?: SemVer }>();
const emit = defineEmits<{ (e: "commit", id: string, name?: string, tag?: string): void }>();

const committed = computed(() => props.version != null && props.version.committed);
const canCommit = computed(() => !committed.value);
// if we can't commit, we're editing an already committed version

const suggestedName = getRandomName();
const name: Ref<string> = ref(props.version?.name ?? suggestedName);
const suggestedTag = renderSemVer(bumpSemVer(props.prevSemVerTag ?? FIRST_SEMVER, "minor"));
const tag: Ref<string> = ref(props.version?.tag ?? suggestedTag);
const description: Ref<string> = ref(props.version?.description ?? "");
const validTag = computed(() => parseSemVer(tag.value) != null);

const operations = useOperations();
</script>

<template>
  <Popover v-slot="{ open, close }" class="relative text-sm">
    <slot name="button" :open="open" />

    <FadeTransition>
      <PopoverPanel
        class="absolute top-6 left-1 z-10 flex w-96 flex-col gap-2 rounded-sm bg-white px-4 py-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
        unmount
      >
        <!-- Header -->
        <div class="">
          <h2 class="font-bold text-gray-900">{{ committed ? "Update snapshot" : "Create a snapshot" }}</h2>
          <p class="mt-1 text-sm text-gray-700">Snapshots are named versions of your Bench.</p>
        </div>

        <!-- Commit name & tag -->
        <div class="mt-2 flex w-full flex-col">
          <span class="text-gray-700">Name & tag</span>
          <div class="flex w-full flex-row rounded-sm border border-gray-200 focus-within:border-orange-600">
            <input
              ref="nameRef"
              type="text"
              minlength="3"
              maxlength="128"
              :placeholder="suggestedName"
              v-model="name"
              class="flex-1 rounded-l-sm border-0 py-1 text-sm placeholder:text-gray-400 focus:bg-orange-50 focus:outline-none focus:ring-0"
              spellcheck="false"
            />
            <div class="relative flex flex-row">
              <TagIcon class="absolute top-1.5 left-2.5 h-4 w-4 text-gray-700" />
              <input
                ref="slugRef"
                type="text"
                minlength="3"
                maxlength="32"
                :placeholder="suggestedTag"
                v-model="tag"
                class="w-24 rounded-r-sm border-0 py-1 pl-8 text-sm placeholder:text-gray-400 focus:bg-orange-50 focus:outline-none focus:ring-0"
                spellcheck="false"
              />
            </div>
          </div>
          <!-- Validation messages -->
          <div class="text-left">
            <FadeTransition mode="out-in">
              <span class="mt-1 text-sm text-yellow-500" v-if="!validTag">That's not a SemVer tag.</span>
            </FadeTransition>
            <p></p>
          </div>
        </div>
        <!-- Commit description -->
        <div class="mt-2 flex w-full flex-col">
          <span class="text-gray-700">Description</span>
          <textarea
            ref="descriptionRef"
            v-model="description"
            class="rounded-sm border border-gray-200 py-1 text-sm placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-50 focus:outline-none focus:ring-0"
            spellcheck="false"
            rows="3"
            placeholder="Optional details for future you."
          />
        </div>

        <!-- Commit / update action -->
        <div class="mt-4 text-right" v-if="canCommit">
          <button
            class="w-fit self-end border border-orange-600 px-3 py-1 text-sm hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
            :class="{ 'pointer-events-none opacity-50': !canCommit }"
            @click="
              emit('commit', version.id, name, tag);
              close();
            "
          >
            Snapshot
          </button>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
