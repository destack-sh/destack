<script lang="ts" setup>
import StatementActions from "@/components/statements/StatementActions.vue";
import StatementTags from "@/components/statements/StatementTags.vue";
import type { StatementAction } from "@/state/bench";
import { useCurrentModule, useNavigation } from "@/state/module";
import { getStatementIconSolid, useStatementContext } from "@/state/statement";
import { TagIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref } from "vue";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import uFuzzy from "@leeoniya/ufuzzy";
import { useOperations } from "@/state/operations";
import type { Statement } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { onStartTyping, useKeyModifier } from "@vueuse/core";

const props = defineProps<{ folded?: boolean }>();
const emit = defineEmits<{ (e: "toggleFold"): void }>();
const context = useStatementContext();
const module = useCurrentModule();
const ops = useOperations();
const appearance = useAppearance();
const nav = useNavigation();

const altKeyState = useKeyModifier("Alt");
const referenceRef = ref<HTMLButtonElement | null>(null);
const tagsRef = ref<InstanceType<typeof StatementTags> | null>(null);
const popoverRef = ref<HTMLDivElement | null>(null);
const inputRef = ref<InstanceType<typeof ComboboxInput> | null>(null);
const popoverPin = pinAbsoluteElement(
  computed(() => popoverRef.value),
  { pos: true, width: true, keepInView: true }
);

const selectingReference = ref(false);
const query = ref("");
const resolvedReference = computed(() => module.statementOf(context.statement.value.reference?.id));
const icon = computed(() => getStatementIconSolid(context.statement.value.type, context.statement.value.rootTypeTag));
const referenceIcon = computed(() =>
  resolvedReference.value == null
    ? null
    : getStatementIconSolid(resolvedReference.value?.type, resolvedReference.value?.rootTypeTag)
);

onStartTyping(() => {
  if (selectingReference.value || !context.editing.value) return;
  open();
});

function open() {
  selectingReference.value = true;
  nextTick(() => inputRef.value?.$el.focus());
}

function close() {
  selectingReference.value = false;
  referenceRef.value?.focus();
}

const uf = new uFuzzy({ intraMode: 0 });
const filteredReferences = computed(() => {
  const candidates = Object.values(module.idx.value?.statementsById ?? {}).filter((s) => (s.name ?? "").trim() != "");
  if (query.value.trim() == "") return candidates;
  const [idxs] = uf.search(
    candidates.map((s) => s.name ?? ""),
    query.value
  );
  return idxs?.map((idx) => candidates[idx]) ?? [];
});

function setReference(statement: Pick<Statement, "id">) {
  ops.symbol.updateStatementReference(
    null,
    context.statement.value.id,
    context.statement.value.reference?.id,
    statement.id
  );
}

function focus(position: "first" | "last" = "first") {
  referenceRef.value?.focus();
}

function blur() {
  referenceRef.value?.blur();
  selectingReference.value = false;
}

const actions = computed(() => {
  const actions: StatementAction[] = [];
  actions.push({
    label: "Add tag",
    icon: TagIcon,
    action: () => {
      tagsRef.value?.open();
    },
  });
  return actions;
});
context.setCustomActions(actions);

defineExpose({
  focus,
  blur,
});
</script>
<template>
  <div class="relative flex flex-row justify-between">
    <!-- Declaration -->
    <div class="flex flex-row gap-1">
      <component :is="icon" class="mt-0.5 h-4 w-4 text-orange-600" />
      <component v-if="referenceIcon" :is="referenceIcon" class="mt-0.5 h-4 w-4 text-orange-600" />
      <button
        ref="referenceRef"
        tabindex="-1"
        @click="altKeyState && resolvedReference != null ? nav.focusStatement(resolvedReference) : open()"
        @keydown.enter.exact.stop.prevent="open"
        @keydown.up.stop.prevent="context.navigateUp"
        @keydown.down.stop.prevent="context.navigateDown"
        class="mb-1 whitespace-nowrap px-0.5 font-semibold text-orange-600 decoration-gray-900 underline-offset-4 focus:bg-orange-100 focus:outline-none focus:ring-0"
        :class="altKeyState ? 'hover:underline' : 'hover:bg-orange-100'"
      >
        {{ resolvedReference?.name ?? (context.statement.value.reference == null ? "..." : "???") }}
      </button>
      <StatementTags ref="tagsRef" class="ml-0.5" />
    </div>
    <!-- Controls -->
    <div
      class="flex flex-row items-center gap-1 transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value ? '' : 'opacity-0'"
    >
      <StatementActions :extra-actions="actions" />
    </div>
    <!-- Prevent scroll and capture click outside -->
    <div
      v-if="selectingReference"
      class="fixed left-0 top-0 z-40 h-full w-full overscroll-none"
      @click.stop="close()"
    />
    <div
      v-if="selectingReference"
      ref="popoverRef"
      class="z-50 flex w-72 flex-col rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      :class="[popoverPin.pinned.value ? '' : 'absolute -top-9 left-5']"
    >
      <Combobox
        as="div"
        @update:model-value="(r) => (setReference(r), close())"
        :class="{ 'font-mono': appearance.fontMono, 'text-sm': appearance.textSmall, 'text-md': !appearance.textSmall }"
      >
        <!-- Title -->
        <h5 class="text-left text-sm font-semibold text-gray-900">Set statement reference</h5>
        <!-- Input -->
        <ComboboxInput
          as="input"
          ref="inputRef"
          class="mt-1 w-full rounded-sm border border-orange-900 border-opacity-[12%] bg-orange-100 p-1 text-gray-900 outline-none ring-0 hover:bg-orange-100 focus:border-orange-900 focus:border-opacity-[12%] focus:ring-0"
          @change="query = $event.target.value"
          @keydown.enter.prevent.stop="close"
          @keydown.escape.prevent.stop="close"
          :class="{
            'font-mono': appearance.fontMono,
            'text-sm': appearance.textSmall,
            'text-md': !appearance.textSmall,
          }"
        >
        </ComboboxInput>
        <!-- Reference options -->
        <ComboboxOptions
          class="mt-1 max-h-48 overflow-auto"
          static
          :class="{
            'font-mono': appearance.fontMono,
            'text-sm': appearance.textSmall,
            'text-md': !appearance.textSmall,
          }"
        >
          <ComboboxOption
            v-for="reference in filteredReferences"
            :key="reference.id"
            :value="reference"
            v-slot="{ active, selected }"
          >
            <li
              :class="[
                'relative flex cursor-default select-none flex-col px-1 py-[3px] text-gray-900',
                active ? 'bg-orange-100' : '',
                selected ? 'text-orange-600' : '',
              ]"
            >
              <div class="flex items-baseline justify-between">
                <span class="flex flex-row items-center">
                  <component
                    :is="getStatementIconSolid(reference.type, reference.rootTypeTag)"
                    class="h-4 w-4 text-orange-600"
                  />
                  <span class="ml-1 font-semibold text-orange-600">{{ reference.name }}</span>
                </span>
                <!-- Source -->
                <span class="text-xs" :class="['truncate', active ? 'text-gray-700' : 'text-gray-500']">
                  {{ module.pathOf(reference.file) }}
                </span>
              </div>
            </li>
          </ComboboxOption>
        </ComboboxOptions>
      </Combobox>
    </div>
  </div>
</template>
