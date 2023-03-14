<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import Switch from "@/components/basic/Switch.vue";
import { getRandomName } from "@/composables/useRandomName";
import { graphql } from "@/gql";
import type { ProjectVersion } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { bumpSemVer, FIRST_SEMVER, parseSemVer, renderSemVer, type SemVer } from "@/utils/semver";
import { Popover, PopoverPanel } from "@headlessui/vue";
import { TagIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useDebounceFn } from "@vueuse/shared";
import { computed, ref, watch, type Ref } from "vue";

const props = defineProps<{ version: ProjectVersion; projectId: string; prevSemVerTag?: SemVer }>();
const emit = defineEmits<{
  (
    e: "commit",
    c: {
      projectVersionId: string;
      name?: string;
      tag?: string;
      description?: string;
      autoDeploy?: boolean;
    }
  ): void;
}>();

// use reference to element inside PopoverPanel to determine if it's open
const panelHeaderRef = ref<HTMLDivElement | null>(null);

const committed = computed(() => props.version != null && props.version.committed);

const suggestedName = getRandomName();
const name: Ref<string> = ref(props.version?.name ?? (committed.value ? "" : suggestedName));
const suggestedTag = renderSemVer(bumpSemVer(props.prevSemVerTag ?? FIRST_SEMVER, "minor"));
const tag: Ref<string> = ref(props.version?.tag ?? suggestedTag);
const description: Ref<string> = ref(props.version?.description ?? "");
const autoDeploy = ref(true);
const canAutoDeploy = computed(() => tag.value.length > 0);

const validTag = computed(() => parseSemVer(tag.value) != null);
// check whether the entered tag is available
const { result: existingTagResult, loading: tagLoading } = useQuery(
  graphql(/* GraphQL */ `
    query existingProjectVersionTag($projectId: GlobalID!, $tag: String!) {
      projectVersionByTag(projectId: $projectId, tag: $tag) {
        id
        tag
      }
    }
  `),
  computed(() => ({
    projectId: props.projectId,
    tag: tag.value,
  })) as any,
  {
    fetchPolicy: "no-cache",
    // only check if we're open
    enabled: computed(() => panelHeaderRef.value != null && tag.value.length > 0),
  }
);
const availableTag = computed(
  () =>
    tag.value.length == 0 ||
    existingTagResult.value?.projectVersionByTag == null ||
    existingTagResult.value?.projectVersionByTag?.id == props.version?.id
);
const canCommit = computed(() => !tagLoading.value && availableTag.value);

// if we can't commit, we're editing an already committed version
// so auto-sync name, description and tag (debounced as usual)
const operations = useOperations();
function updateVersion() {
  operations.version.update(
    props.version.id,
    name.value,
    tag.value.length > 0 ? tag.value : undefined,
    description.value
  );
}
const updateVersionDebounced = useDebounceFn(updateVersion, 500);
watch([name, description, tag, availableTag, tagLoading], () => {
  if (availableTag.value && !tagLoading.value) {
    updateVersionDebounced();
  }
});
</script>

<template>
  <Popover v-slot="{ open, close }" class="relative text-sm">
    <slot :open="open" />

    <FadeTransition>
      <PopoverPanel
        class="absolute top-9 left-1 z-10 flex w-96 flex-col gap-2 rounded-sm bg-white px-4 py-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
        unmount
      >
        <!-- Header -->
        <div class="" ref="panelHeaderRef">
          <h2 class="font-bold text-gray-900">{{ committed ? "Update snapshot" : "Create a snapshot" }}</h2>
          <p v-if="!committed" class="mt-1 text-sm text-gray-700">Snapshots are named versions of your Bench.</p>
        </div>

        <!-- Commit name & tag -->
        <div class="mt-2 flex w-full flex-col">
          <span class="text-gray-700">Name & tag</span>
          <div
            class="flex w-full flex-row rounded-sm border border-orange-900 border-opacity-[12%] focus-within:border-orange-600"
          >
            <input
              ref="nameRef"
              type="text"
              minlength="3"
              maxlength="128"
              :placeholder="suggestedName"
              v-model="name"
              class="flex-1 rounded-l-sm border-0 py-1 text-sm placeholder:text-gray-400 focus:bg-orange-100 focus:outline-none focus:ring-0"
              spellcheck="false"
            />
            <div class="relative flex flex-row">
              <TagIcon
                class="absolute top-1.5 left-2.5 h-4 w-4"
                :class="tag.length > 0 ? 'text-gray-700' : 'text-gray-400'"
              />
              <input
                ref="slugRef"
                type="text"
                minlength="3"
                maxlength="32"
                :placeholder="suggestedTag"
                v-model="tag"
                class="w-28 rounded-r-sm border-0 py-1 pl-8 text-sm placeholder:text-gray-400 focus:bg-orange-100 focus:outline-none focus:ring-0"
                :class="{ 'text-yellow-600': !validTag, 'text-red-600': !availableTag }"
                spellcheck="false"
              />
            </div>
          </div>
        </div>
        <!-- Commit description -->
        <div class="mt-2 flex w-full flex-col">
          <span class="text-gray-700">Description</span>
          <textarea
            ref="descriptionRef"
            v-model="description"
            class="rounded-sm border border-orange-900 border-opacity-[12%] py-1 text-sm placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-100 focus:outline-none focus:ring-0"
            spellcheck="false"
            rows="3"
            placeholder="Optional details for future you."
          />
        </div>
        <!-- Validation messages -->
        <div class="text-left">
          <FadeTransition mode="out-in">
            <span class="mt-1 text-sm text-red-600" v-if="!availableTag">That tag is already used.</span>
            <span class="mt-1 text-sm text-yellow-600" v-else-if="tag.length > 0 && !validTag"
              >We recommend SemVer tags - they play nicely.</span
            >
            <span v-else>&nbsp;</span>
          </FadeTransition>
          <p></p>
        </div>

        <!-- Deployment (if head) -->
        <div v-if="!committed" class="flex flex-row items-baseline justify-between gap-1">
          <p class="flex-1 whitespace-nowrap">
            <FadeTransition mode="out-in">
              <span v-if="tag.length > 0" class="text-gray-700">
                <span class="font-bold">Auto-deploy</span> this tagged version.
              </span>
              <span v-else class="text-yellow-600">Version without a tag cannot be deployed.</span>
            </FadeTransition>
          </p>
          <Switch v-model="autoDeploy" v-if="canAutoDeploy" />
        </div>

        <!-- Commit / update action -->
        <div class="mt-4 text-right" v-if="!committed">
          <button
            :disabled="!canCommit"
            class="w-fit self-end border border-orange-600 px-3 py-1 text-sm hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
            :class="{ 'pointer-events-none opacity-50': !canCommit }"
            @click="
              emit('commit', {
                projectVersionId: version.id,
                name,
                tag,
                description,
                autoDeploy: canAutoDeploy && autoDeploy,
              });
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
