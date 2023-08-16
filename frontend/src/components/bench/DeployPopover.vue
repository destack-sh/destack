<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useFragment, type FragmentType } from "@/gql";
import { ProjectHeaderType } from "@/state/fragments";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { CloudIcon, DocumentDuplicateIcon } from "@heroicons/vue/24/outline";
import { useClipboard } from "@vueuse/core";
import { computed } from "vue";

const props = defineProps<{ project: FragmentType<typeof ProjectHeaderType> }>();

const project = computed(() => useFragment(ProjectHeaderType, props.project));

const clipboard = useClipboard();
function copyApiUrlToClipboard() {
  const url = `https://api.bench.is/${project.value.owner.slug}/${project.value.slug}/run`;
  clipboard.copy(url);
}
</script>

<template>
  <Popover v-slot="{ open }" class="relative">
    <PopoverButton
      ref="deployButtonRef"
      class="relative rounded-sm p-1 text-sm focus:outline-none"
      :class="{
        'text-orange-600 hover:bg-orange-100': true,
        'bg-orange-100': open,
      }"
    >
      <CloudIcon class="h-5 w-5" />
    </PopoverButton>

    <FadeTransition>
      <PopoverPanel
        class="absolute right-0 top-10 z-30 mt-0 flex w-96 flex-col gap-2 rounded-sm bg-white px-4 pb-4 pt-2 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <h2 class="font-bold text-gray-900">Integrate your Bench</h2>
        <p class="pt-2 text-gray-900">
          The current version is tagged
          <span class="rounded-sm bg-gray-200 px-0.5 font-mono">x</span>).
        </p>
        <p class="pt-0 text-gray-900">
          Endpoints are available via
          <router-link
            to="/symbolx/docs#Deploying"
            target="_blank"
            class="underline decoration-gray-500 decoration-dashed underline-offset-4 hover:decoration-solid"
            >REST</router-link
          >
          at:
        </p>
        <p class="relative mt-2 w-full rounded-sm border border-orange-900 border-opacity-[20%] p-1">
          <span :href="`https://api.bench.is/${project.owner.slug}/${project.slug}/run`" class="text-gray-900">
            api.bench.is/<span class="text-orange-600">{{ project.owner.slug }}</span
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
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
