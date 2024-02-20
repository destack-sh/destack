<script lang="ts" setup>
import { useAppearance } from "@/state/appearance";
import {
  useCurrentModule,
  TypeFlag,
  type Field,
  useNavigation,
  type NodeBase,
  type InterpStatement,
} from "@/state/module";
import { computed, type Ref, ref } from "vue";
import { Combobox, ComboboxButton, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { getStatementIconSolid } from "@/state/statement";
import uFuzzy from "@leeoniya/ufuzzy";
import { TypeHint } from "@/gql/graphql";
import { CodeBracketIcon } from "@heroicons/vue/24/outline";

const props = defineProps<{
  type: Field;
  modelValue?: string[];
  readonly?: boolean;
  preview?: boolean;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: string[]): void;
  (e: "close"): void;
}>();

const module = useCurrentModule();
const nav = useNavigation();
const isArray = computed(() => Boolean(props.type.flags & TypeFlag.IS_ARRAY));

const selectedNodes: Ref<NodeBase[]> = computed(
  () => (props.modelValue?.map((v) => module.nodeOf(v)).filter((v) => v != null) as NodeBase[]) ?? []
);
const nodes: Ref<(NodeBase & { path?: string })[]> = computed(() => {
  if (props.preview) return [];
  if (props.type.hint == TypeHint.Statement) {
    return (module.namedStatements.value as NodeBase[]).map((n) => ({
      ...n,
      path: module.pathOf(n.id, { roffset: 1 }),
    }));
  } else if (props.type.hint == TypeHint.File) {
    return (module.namedFiles.value as NodeBase[]).map((n) => ({
      ...n,
      path: module.pathOf(n.id, { roffset: 1 }),
    }));
  } else {
    throw new Error("unexpected node type hint");
  }
});
const missingNodes: Ref<(NodeBase & { path?: string })[]> = computed(() => {
  if (props.preview) return [];
  return nodes.value.filter((m) => !props.modelValue?.find((v) => v == m.ck));
});
const query = ref("");
const uf = new uFuzzy({ intraMode: 0 });
const filteredNodes = computed(() => {
  const baseNodes = isArray.value ? missingNodes.value : nodes.value;
  if (query.value.trim() == "") return baseNodes;
  const [idxs, info, order] = uf.search(
    baseNodes.map((m) => m.name ?? ""),
    query.value,
    true
  );
  if (idxs && order) {
    return order.map((i) => baseNodes[idxs[i]]);
  }
  return baseNodes;
});

function getNodeIcon(node: NodeBase) {
  if (node.__typename == "Statement") {
    return getStatementIconSolid((node as InterpStatement).type);
  } else if (node.__typename == "File") {
    return CodeBracketIcon;
  } else {
    throw new Error("unexpected node type");
  }
}

const queryRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);

function removeValue(key: string) {
  emit("update:modelValue", props.modelValue?.filter((v) => v != key) ?? []);
}

const appearance = useAppearance();

defineExpose({
  focus: () => queryRef.value?.$el.focus(),
  blur: () => queryRef.value?.$el.blur(),
});
</script>
<template>
  <div class="flex h-full w-full flex-row flex-wrap gap-1">
    <!-- :EnumStyle -->
    <!-- Existing nodes -->
    <span
      v-for="node in selectedNodes"
      :key="node.ck"
      class="inline-flex items-center gap-x-1 whitespace-nowrap rounded-sm text-gray-900 hover:cursor-pointer hover:bg-amber-100"
    >
      <component :is="getNodeIcon(node)" class="h-4 w-4 text-orange-600" />
      <span class="underline decoration-gray-300 underline-offset-4">
        {{ node.name }}
      </span>
      <!-- Delete button -->
      <button
        v-if="!preview && !readonly"
        class="p-0.5 text-gray-300 hover:text-gray-700"
        @click.stop="removeValue(node.ck)"
      >
        x
      </button>
    </span>
    <!-- Ensure there's always something -->
    <template v-if="selectedNodes.length == 0 && preview">&nbsp;</template>
    <Combobox
      v-if="!preview"
      as="div"
      class="flex w-full min-w-[300px] flex-col"
      :model-value="isArray ? null : selectedNodes[0]"
      @update:model-value="(val: Field) => {
        query = '';
        if (isArray) {
          emit('update:modelValue', [...(modelValue ?? []), val.ck as string]);
        } else {
          emit('update:modelValue', [val.ck as string]);
          emit('close')
        }
      }"
    >
      <!-- Hidden button to manage focus programmatically -->
      <ComboboxButton class="hidden" ref="comboboxButtonRef" />
      <ComboboxInput
        as="input"
        ref="queryRef"
        :default-value="query"
        @change="query = $event.target.value"
        spellcheck="false"
        class="mx-1.5 mt-1 w-full min-w-0 rounded-none border-none bg-transparent p-0 outline-none ring-0 placeholder:text-gray-400 focus:ring-0"
        :class="[appearance.textSmall ? 'text-sm' : '', isArray ? 'mt-1' : '']"
        :display-value="(val: any) => ''"
        :placeholder="(isArray ? 'Add ' : 'Select ') + (type.hint ?? '').toLowerCase()"
        @keydown.backspace.exact="
          queryRef?.$el.value.length > 0 || removeValue(modelValue?.[modelValue.length - 1] ?? '')
        "
      >
      </ComboboxInput>
      <ComboboxOptions
        class="mt-1 flex max-h-80 w-full flex-col gap-y-0.5 overflow-auto border-t py-1 pt-1.5 focus:outline-none"
        static
      >
        <ComboboxOption
          v-for="node in filteredNodes"
          :key="node.name ?? ''"
          :value="node"
          v-slot="{ active, selected }"
        >
          <div
            :class="[
              'relative w-full cursor-default select-none hover:cursor-pointer ',
              selected ? 'text-amber-600' : 'text-gray-900',
            ]"
          >
            <li
              class="mx-1 flex w-full max-w-full flex-row items-center gap-1.5 whitespace-nowrap px-1.5 py-0.5"
              :class="[active ? 'bg-amber-100' : '']"
            >
              <component :is="getNodeIcon(node)" class="h-4 w-4 text-orange-600" />
              <span class="truncate">{{ node.name }}</span>
              <span class="ml-auto text-xs text-gray-400">{{ node.path }}</span>
            </li>
          </div>
        </ComboboxOption>
      </ComboboxOptions>
    </Combobox>
  </div>
</template>
