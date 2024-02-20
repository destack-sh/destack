<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { getRandomName } from "@/composables/useRandomName";
import { graphql } from "@/gql";
import type { ProjectVersion } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { bumpSemVer, FIRST_SEMVER, parseSemVer, renderSemVer, type SemVer } from "@/utils/semver";
import { VALID_NAME_CHAR_REGEX, VALID_NAME_CHAR_REGEXP, VALID_NAME_REGEX } from "@/utils/validation";
import { Popover, PopoverPanel } from "@headlessui/vue";
import { TagIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useDebounceFn } from "@vueuse/shared";
import { computed, ref, watch, type Ref } from "vue";

const props = defineProps<{ version: ProjectVersion; projectId: string; isHead?: boolean; prevSemVerTag?: SemVer }>();
const emit = defineEmits<{
  (
    e: "snapshot",
    c: {
      projectVersionId: string;
      name?: string;
      tag?: string;
      description?: string;
    }
  ): void;
}>();

// use reference to element inside PopoverPanel to determine if it's open
const panelHeaderRef = ref<HTMLDivElement | null>(null);
const panelRef = ref<InstanceType<typeof PopoverPanel> | null>(null);
const panelRefPin = pinAbsoluteElement(
  computed(() => panelRef.value?.$el),
  { pos: true }
);
const nameRef = ref<HTMLInputElement | null>(null);

const committed = computed(() => props.version != null && props.version.committed);

function suggestTag(): string {
  return renderSemVer(bumpSemVer(props.prevSemVerTag ?? FIRST_SEMVER, "patch"));
}

const suggestedName = getRandomName();
const suggestedTag = suggestTag();
const name: Ref<string> = ref(props.version?.name ?? (committed.value ? "" : suggestedName));
const tag: Ref<string> = ref(props.version?.tag ?? suggestedTag);
const description: Ref<string> = ref(props.version?.description ?? "");

// reset name and tag when popover is opened
watch(
  () => panelHeaderRef.value,
  () => {
    if (panelHeaderRef.value != null) {
      name.value = props.version?.name ?? (committed.value ? "" : getRandomName());
      tag.value = props.version?.tag ?? suggestTag();
      description.value = props.version?.description ?? "";
    }
  }
);

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
    enabled: computed(() => panelHeaderRef.value != null && tag.value.length > 0) as any,
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
const ops = useOperations();
function updateVersion() {
  ops.version.update(props.version.id, name.value, tag.value.length > 0 ? tag.value : undefined, description.value);
}
const updateVersionDebounced = useDebounceFn(updateVersion, 500);
watch([name, description, tag, availableTag, tagLoading, () => props.isHead], () => {
  if (availableTag.value && !tagLoading.value && !props.isHead) {
    updateVersionDebounced();
  }
});

function snapshot() {
  emit("snapshot", {
    projectVersionId: props.version.id,
    name: name.value,
    tag: tag.value,
    description: description.value,
  });
}

defineExpose({
  focus: () => {
    nameRef.value?.focus();
  },
});
</script>

<template>
  <Popover v-slot="{ open, close }" class="relative text-sm">
    <slot :open="open" />

    <FadeTransition>
      <PopoverPanel
        ref="panelRef"
        class="z-40 flex w-96 flex-col gap-2 rounded-sm bg-white px-4 py-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :class="[panelRefPin.pinned.value ? '' : 'absolute left-1 top-9']"
        unmount
      >
        <!-- Header -->
        <div class="" ref="panelHeaderRef">
          <h2 class="font-bold text-gray-900">{{ committed ? "Update snapshot" : "Create a snapshot" }}</h2>
          <p v-if="!committed" class="mt-1 text-sm text-gray-700">
            Snapshots are named versions of your Bench. You can restore everything or specific parts at any time.
          </p>
        </div>

        <!-- Commit name & tag -->
        <div class="mt-2 flex w-full flex-col">
          <span class="text-gray-700">Name & tag</span>
          <div class="flex w-full flex-row rounded-sm border border-orange-900/[12%] focus-within:border-orange-600">
            <input
              ref="nameRef"
              type="text"
              minlength="3"
              maxlength="128"
              :placeholder="suggestedName"
              v-model="name"
              :pattern="VALID_NAME_CHAR_REGEX"
              class="flex-1 rounded-l-sm border-0 py-1 text-sm placeholder:text-gray-400 focus:bg-orange-100 focus:outline-none focus:ring-0"
              spellcheck="false"
              @keydown.ctrl.enter.exact.prevent="snapshot(), close()"
            />
            <div class="relative flex flex-row">
              <TagIcon
                class="absolute left-2.5 top-1.5 h-4 w-4"
                :class="tag.length > 0 ? 'text-gray-700' : 'text-gray-400'"
              />
              <input
                ref="slugRef"
                type="text"
                minlength="3"
                maxlength="32"
                :placeholder="suggestedTag"
                :pattern="VALID_NAME_CHAR_REGEX"
                v-model="tag"
                class="w-28 rounded-r-sm border-0 py-1 pl-8 text-sm placeholder:text-gray-400 focus:bg-orange-100 focus:outline-none focus:ring-0"
                :class="{ 'text-yellow-600': !validTag, 'text-red-600': !availableTag }"
                spellcheck="false"
                @keydown.ctrl.enter.exact.prevent="snapshot(), close()"
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
            class="resize-none rounded-sm border border-orange-900/[12%] py-1 text-sm placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-100 focus:outline-none focus:ring-0"
            spellcheck="false"
            rows="3"
            placeholder="Optional details for future you."
            @keydown.ctrl.enter.exact.prevent="snapshot(), close()"
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
        <!-- Commit / update action -->
        <div class="mt-4 text-right" v-if="!committed">
          <button
            :disabled="!canCommit"
            class="w-fit self-end border border-orange-600 px-3 py-1 text-sm hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
            :class="{ 'pointer-events-none opacity-50': !canCommit }"
            @click="
              emit('snapshot', {
                projectVersionId: version.id,
                name,
                tag,
                description,
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
