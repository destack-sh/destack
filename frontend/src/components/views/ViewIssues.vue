<script lang="ts" setup>
import { IssueKind, type IssueContentFragment } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { useCurrentModule, useNavigation } from "@/state/module";
import { FaceSmileIcon } from "@heroicons/vue/24/outline";
import { computed } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { ExclamationTriangleIcon, XCircleIcon } from "@heroicons/vue/24/solid";

const appearance = useAppearance();
const module = useCurrentModule();
const nav = useNavigation();
const issues = computed(() => module.issues.value);

function focusIssue(issue: IssueContentFragment) {
  if (issue.parent?.__typename == "Statement") {
    nav.focusStatement(issue.parent);
  } else if (issue.parent?.__typename == "File") {
    nav.focusFile(issue.parent);
  }
}
</script>
<template>
  <div class="">
    <!-- View header -->
    <div
      class="flex h-[31px] flex-row items-center justify-between px-3 py-2"
      :style="{
        height: appearance.panelHeaderHeight + 'px',
      }"
    >
      <span class="text-xs font-semibold tracking-wide text-gray-500">Issues</span>
      <div v-if="module.loading.value">
        <BusySpinnerIcon class="h-4 w-4 animate-spin text-gray-500" />
      </div>
    </div>
    <ul class="flex w-full flex-col gap-2 overflow-y-auto py-2 pb-10" v-if="!module.loading.value">
      <!-- This is pretty crude, should probably be grouped by location/type/severity/whatever -->
      <li
        v-for="(issue, i) in issues ?? []"
        :key="i"
        class="group flex flex-col justify-between py-0.5 text-sm hover:cursor-pointer hover:bg-orange-100"
        @click="focusIssue(issue)"
      >
        <!-- Location -->
        <span class="flex max-w-full flex-row items-baseline gap-x-1.5 gap-y-0.5 px-3">
          <!-- Name -->
          <span class="flex-shrink-0 truncate">
            {{ (issue.parent == null ? null : module.nodeOf(issue.parent.id))?.name ?? "(text)" }}
          </span>
          <!-- Location -->
          <span class="truncate text-sm text-gray-500" v-if="issue.parent != null">
            {{
              module
                .nodePathOf(issue.parent.id)
                ?.slice(0, -1)
                ?.map((e) => e.name)
                .join(".")
            }}
          </span>
        </span>
        <!-- Issue description -->
        <span
          class="flex flex-row gap-1 px-3"
          :class="{
            'text-red-600': issue.kind == IssueKind.Error,
            'text-yellow-600': issue.kind == IssueKind.Warning,
            'text-cyan-600': issue.kind == IssueKind.Notice,
          }"
        >
          <component
            :is="issue.kind == IssueKind.Error ? XCircleIcon : ExclamationTriangleIcon"
            class="mt-0.5 h-4 w-4 flex-shrink-0"
          />
          <span class="">{{ issue.message }}</span>
        </span>
      </li>
      <div v-if="issues.length == 0" class="my-4 flex flex-col items-center justify-center gap-2 px-3 text-center">
        <FaceSmileIcon class="h-7 w-7 text-gray-500" />
        <span class="text-sm text-gray-700">A tidy Bench. The bots like it.</span>
      </div>
    </ul>
  </div>
</template>
