<script lang="ts" setup>
import { useAppearance } from "@/state/appearance";
import { newNodeIdentity, useCurrentModule } from "@/state/module";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { TagIcon as TagIconOutline } from "@heroicons/vue/24/outline";
import { TagIcon as TagIconSolid } from "@heroicons/vue/24/solid";
import { computed, nextTick, ref, toRef } from "vue";
import uFuzzy from "@leeoniya/ufuzzy";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { useOperations } from "@/state/operations";
import type { Tagging } from "@/gql/graphql";
import type { Statement } from "@/gql/graphql";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { useTags } from "@/state/statement";
import { useElementRefs } from "@/composables/useGrid";
import type { StatementAction } from "@/state/bench";
import { IS_DEBUG } from "@/utils/globals";

const props = defineProps<Pick<StatementProps, "statement" | "readonly">>();
const emit = defineEmits<StatementEmit>();

const { tags } = useTags(toRef(props, "statement"));
const module = useCurrentModule();
const appearance = useAppearance();
const ops = useOperations();

const addingTag = ref(false);
const popoverRef = ref<HTMLDivElement | null>(null);
const inputRef = ref<InstanceType<typeof ComboboxInput> | null>(null);
const query = ref("");
const popoverPin = pinAbsoluteElement(
  computed(() => popoverRef.value),
  { pos: true, width: true, keepInView: true }
);
const taggingRefs = useElementRefs<HTMLButtonElement>();

const uf = new uFuzzy({ intraMode: 0 });
const filteredTags = computed(() => {
  const unassigned = module.tags.value.filter((t) => !tags.value.some((t2) => t2.key == t.key));
  if (query.value.trim() == "") return unassigned;
  const [idxs, info, order] = uf.search(
    unassigned.map((t) => t.name ?? ""),
    query.value
  );
  if (idxs && order) {
    return order.map((i) => unassigned[idxs[i]]);
  }
  return unassigned;
});

function open() {
  addingTag.value = true;
  nextTick(() => inputRef.value?.$el.focus());
}

function close() {
  addingTag.value = false;
}

function createTagging(tag: Pick<Statement, "ck" | "key" | "name">) {
  const identity = newNodeIdentity(module.id.value, "Tagging");
  ops.symbol.createTagging(null, props.statement.id, {
    id: identity.id,
    ck: identity.ck,
    key: tag.key as string,
    referenceCk: tag.ck,
    value: null,
  });
  nextTick(() => taggingRefs.focus(identity.id));
}

function deleteTagging(tagging: Pick<Tagging, "id">) {
  ops.symbol.softDeleteTagging(null, props.statement.id, tagging);
}

defineExpose({
  focus: (focus: "first" | "last" = "first") => {
    if (focus == "first") {
      taggingRefs.focus(tags.value[0]?.id);
    } else {
      taggingRefs.focus(tags.value[tags.value.length - 1]?.id);
    }
  },
  blur: () => {
    taggingRefs.refs.value.forEach((ref) => ref.blur?.());
    close();
  },
  actions: [
    {
      label: "Add tag",
      groupId: "edit",
      icon: TagIconOutline,
      disabled: props.readonly,
      action: () => open(),
    },
  ] as StatementAction[],
  open,
  close,
});
</script>
<template>
  <div class="group relative flex flex-row gap-1.5">
    <!-- Existing tags -->
    <!-- obviously deleting on click is bad UX and will be fixed when we have proper tag value menus -->
    <button
      v-for="(tagging, i) in tags"
      :ref="(ref: any) => taggingRefs.registerRef(tagging.id, ref)"
      :key="tagging.id"
      class="flex h-fit max-h-fit flex-row rounded-xl px-1 text-emerald-900 ring-inset ring-emerald-600/20 hover:bg-emerald-200 hover:ring-1 focus:bg-emerald-200 focus:outline-none focus:ring-emerald-600/80"
      @click="deleteTagging(tagging)"
      @keydown.delete.exact="deleteTagging(tagging)"
      @keydown.left.exact.prevent="i == 0 ? emit('navigateLeft') : taggingRefs.focus(tags[i - 1]?.id)"
      @keydown.right.exact.prevent="i == tags.length - 1 ? emit('navigateRight') : taggingRefs.focus(tags[i + 1]?.id)"
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
    >
      <TagIconSolid class="mt-0.5 h-4 w-4" />
      <span class="text-orange-00 ml-0.5 font-semibold">
        {{ module.tagsByKey.value[tagging.key]?.name ?? (IS_DEBUG ? tagging.key : "???") }}
      </span>
    </button>
    <!-- Prevent scroll and capture click outside -->
    <div v-if="addingTag" class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="close()" />
    <!-- Add tag popover -->
    <FadeTransition>
      <div
        v-if="addingTag"
        ref="popoverRef"
        class="z-50 flex w-72 flex-col rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :class="[popoverPin.pinned.value ? '' : 'absolute -top-4']"
      >
        <Combobox as="div" @update:model-value="(t) => (close(), createTagging(t))">
          <!-- Input -->
          <ComboboxInput
            as="input"
            ref="inputRef"
            class="mt-1 w-full rounded-sm border border-orange-900/[12%] bg-orange-100 p-1 text-gray-900 outline-none ring-0 hover:bg-orange-100 focus:border-orange-900 focus:border-opacity-[12%] focus:ring-0"
            @change="query = $event.target.value"
            @keydown.enter.prevent.stop="close"
            @keydown.escape.prevent.stop="close"
            :class="{
              'font-mono': appearance.fontMono,
              'text-sm': appearance.textSmall,
              'text-md': !appearance.textSmall,
            }"
          />
          <!-- Tag options -->
          <ComboboxOptions
            class="mt-1 max-h-48 overflow-auto"
            static
            :class="{
              'font-mono': appearance.fontMono,
              'text-sm': appearance.textSmall,
              'text-md': !appearance.textSmall,
            }"
          >
            <ComboboxOption v-for="tag in filteredTags" :key="tag.id" :value="tag" v-slot="{ active, selected }">
              <li
                :class="[
                  'relative flex cursor-default select-none flex-col px-1 py-[3px] text-gray-900',
                  active ? 'bg-orange-100' : '',
                  selected ? 'text-orange-900' : '',
                ]"
              >
                <!-- Tag path -->
                <div class="flex items-center justify-between">
                  <span class="flex flex-row items-center">
                    <TagIconOutline class="h-4 w-4 text-gray-400" />
                    <span class="ml-1 text-gray-900">{{ tag.name }}</span>
                  </span>
                  <!-- Source -->
                  <span class="text-xs" :class="['truncate', active ? 'text-gray-700' : 'text-gray-500']">
                    {{ module.pathOf(tag.file.id) ?? "(builtin)" }}
                  </span>
                </div>
                <!-- Tag description -->
                <span
                  class="ml-5 max-w-full truncate text-xs"
                  :class="['', active ? 'text-gray-700' : 'text-gray-500']"
                >
                  {{ tag.text }}
                </span>
              </li>
            </ComboboxOption>
          </ComboboxOptions>
        </Combobox>
      </div>
    </FadeTransition>
  </div>
</template>
