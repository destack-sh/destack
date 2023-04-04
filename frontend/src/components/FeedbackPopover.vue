<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import Switch from "@/components/basic/Switch.vue";
import { useNotifications } from "@/state/notifications";
import { VERSION } from "@/utils/globals";
import { bumpSemVer, parseSemVer, renderSemVer, type SemVer } from "@/utils/semver";
import { Popover, PopoverPanel } from "@headlessui/vue";
import posthog from "posthog-js";
import { ref, type Ref } from "vue";

const feedback: Ref<string> = ref("");
const getBackToMe: Ref<boolean> = ref(false);
const notifications = useNotifications();

function submit() {
  posthog.capture("feedback", {
    feedback: feedback.value,
    getBackToMe: getBackToMe.value,
  });
  const nextBenchVersion = bumpSemVer(parseSemVer(VERSION) as SemVer, "minor");
  notifications.show({
    kind: "success",
    type: "feedback.success",
    message: "Feedback submitted",
    description: `Thank you! Bench ${renderSemVer(nextBenchVersion)} will be even better.`,
  });
}
</script>

<template>
  <Popover v-slot="{ open, close }" class="relative">
    <slot name="button" :open="open" />

    <FadeTransition>
      <PopoverPanel
        class="absolute bottom-0 left-14 z-10 flex w-72 flex-col gap-2 rounded-sm bg-white p-3 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <!-- Header -->
        <div>
          <h3 class="text-sm font-semibold text-gray-900">Gift feedback</h3>
          <p class="mt-2 text-sm text-gray-700">
            We love feedback - glitches, confusions, bugs, suggestions, anything goes.
          </p>
        </div>
        <!-- Feedback -->
        <div class="mt-2 flex w-full flex-col">
          <textarea
            ref="descriptionRef"
            v-model="feedback"
            class="rounded-sm border border-orange-900 border-opacity-[12%] py-1 text-sm placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-100 focus:outline-none focus:ring-0"
            spellcheck="false"
            rows="3"
            placeholder="This seemed weird / didn't work as expected ..."
          />
        </div>
        <!-- Get back to me? -->
        <div class="mt-2 flex flex-row items-center justify-between">
          <span class="flex flex-row items-center gap-2">
            <span class="text-sm text-gray-900">Get back to me</span>
          </span>
          <Switch v-model="getBackToMe" />
        </div>
        <!-- Submit -->
        <div class="mt-4 text-right">
          <button
            class="w-fit self-end border border-orange-600 px-3 py-1 text-sm hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
            :class="feedback.trim().length == 0 ? 'cursor-not-allowed opacity-50' : ''"
            :disabled="feedback.trim().length == 0"
            @click="
              submit();
              close();
            "
          >
            Submit
          </button>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
