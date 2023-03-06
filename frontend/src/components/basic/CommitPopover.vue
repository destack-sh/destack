<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import type { ProjectVersion } from "@/gql/graphql";
import { Popover, PopoverPanel } from "@headlessui/vue";
import { computed } from "vue";

const props = defineProps<{ version?: ProjectVersion }>();
const emit = defineEmits<{ (e: "commit"): void }>();

const committed = computed(() => props.version != null && props.version.committed);

const canCommit = computed(() => true);
</script>

<template>
  <Popover v-slot="{ open }" class="relative text-sm">
    <slot name="button" :open="open" />

    <FadeTransition>
      <PopoverPanel
        class="absolute top-1 left-14 z-10 flex w-60 flex-col gap-2 rounded-sm bg-white px-2 py-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <!-- Header -->
        <div class="">
          <h2 class="font-bold text-gray-900">{{ committed ? "Update snapshot" : "Create a snapshot" }}</h2>
        </div>

        <!-- Commit name -->
        <!-- Commit tag -->
        <!-- Commit description -->

        <!-- Commit / update action -->
        <div class="mt-4 text-right">
          <button
            class="w-fit self-end border border-orange-600 px-3 py-1 text-sm hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
            :class="{ 'pointer-events-none opacity-50': !canCommit }"
            @click="emit('commit')"
          >
            Snapshot
          </button>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
