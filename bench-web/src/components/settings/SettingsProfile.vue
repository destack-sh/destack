<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useValidName, useValidSlug } from "@/composables/useValidation";
import { graphql } from "@/gql";
import { useOperationsStore } from "@/state/operations";
import { useMutation, useQuery } from "@vue/apollo-composable";
import { useDebounceFn } from "@vueuse/shared";
import { computed, ref, watchEffect, type Ref } from "vue";

const props = defineProps<{ slug: string }>();

const { result: profileResult } = useQuery(
  graphql(/* GraphQL */ `
    query profileSettings($slug: String!) {
      ownerBySlug(slug: $slug) {
        ... on User {
          id
          slug
          name
          username
          description
          canWrite
        }
        ... on Organization {
          id
          slug
          name
          description
          canWrite
        }
      }
    }
  `),
  computed(() => ({ slug: props.slug }))
);

const isOrganization = computed(() => profileResult.value?.ownerBySlug?.__typename === "Organization");
const canWrite = computed(() => profileResult.value?.ownerBySlug?.canWrite);

const name: Ref<string | null> = ref(null);
const description: Ref<string | null> = ref(null);
const slug: Ref<string | null> = ref(null);

const nameValidation = useValidName(name);
const slugValidation = useValidSlug(
  slug,
  computed(() => profileResult.value?.ownerBySlug?.id)
);

// sync fields on loaded
const loaded = ref(false);
watchEffect(() => {
  if (profileResult.value?.ownerBySlug != null && !loaded.value) {
    name.value = profileResult.value.ownerBySlug.name;
    description.value = profileResult.value.ownerBySlug.description ?? "";
    slug.value = profileResult.value.ownerBySlug.slug;
    loaded.value = true;
  }
});

// sync info fields (name & description)
const { mutate: updateOrganizationMut } = useMutation(
  graphql(/* GraphQL */ `
    mutation updateOrganization($id: GlobalID!, $name: String!, $description: String!) {
      updateOrganization(input: { id: $id, name: $name, description: $description }) {
        ... on Organization {
          id
          name
          description
        }
        ...OperationInfoContent
      }
    }
  `)
);
const { mutate: updateUserMut } = useMutation(
  graphql(/* GraphQL */ `
    mutation updateUser($id: GlobalID!, $name: String!, $description: String!) {
      updateUser(input: { id: $id, name: $name, description: $description }) {
        ... on User {
          id
          name
          description
        }
        ...OperationInfoContent
      }
    }
  `)
);

const ops = useOperationsStore();
async function updateInfo(name: string, description: string) {
  const mut = isOrganization.value ? updateOrganizationMut : updateUserMut;
  await ops.perform({
    type: "auth.updateInfo",
    do: async () => {
      return await mut({
        id: profileResult.value?.ownerBySlug?.id,
        name,
        description,
      });
    },
  });
}
// update info debounced on edit
const updateInfoDebounced = useDebounceFn(updateInfo, 500, { maxWait: 2000 });
watchEffect(() => {
  // update once loaded and actually changed
  if (
    (name.value != null && name.value != profileResult.value?.ownerBySlug?.name) ||
    (description.value != null && description.value != (profileResult.value?.ownerBySlug?.description ?? ""))
  ) {
    updateInfoDebounced(name.value ?? "", description.value ?? "");
  }
});
</script>
<template>
  <div>
    <p class="text-gray-900">This is who you really are.</p>

    <div class="mt-4 flex flex-col gap-4" v-show="profileResult?.ownerBySlug != null">
      <!-- Full name -->
      <div class="flex w-96 flex-col text-left">
        <span class="text-md text-gray-700">Name</span>
        <input
          ref="nameRef"
          type="text"
          minlength="3"
          maxlength="128"
          :placeholder="isOrganization ? 'E Corp' : 'Yatima'"
          v-model="name"
          class="mt-1 w-full rounded-sm border border-orange-900 border-opacity-[15%] py-1 placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-100 focus:outline-none focus:ring-0"
          spellcheck="false"
        />
        <FadeTransition mode="out-in">
          <span class="mt-1 text-sm text-yellow-600" v-if="name != null && !nameValidation.valid.value"
            >The bots don't like this name.</span
          >
          <span class="mt-1 text-sm text-gray-500" v-else>Great name.</span>
        </FadeTransition>
      </div>

      <!-- Slug -->
      <div class="flex w-96 flex-col text-left">
        <span class="text-md text-gray-700">Bot-friendly name</span>
        <div class="flex flex-row items-baseline">
          <span
            class="mt-1 rounded-l-sm border border-r-0 border-orange-900 border-opacity-[15%] bg-white px-3 py-1 text-gray-500"
            >bench.is/</span
          >
          <input
            ref="slugRef"
            type="text"
            minlength="3"
            maxlength="128"
            :placeholder="isOrganization ? 'e-corp' : 'yatima'"
            v-model="slug"
            class="mt-1 flex-1 rounded-r-sm border border-orange-900 border-opacity-[15%] py-1 placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-100 focus:outline-none focus:ring-0"
            spellcheck="false"
            disabled
          />
        </div>

        <FadeTransition mode="out-in">
          <span class="mt-1 text-sm text-yellow-600" v-if="slug != null && !slugValidation.valid.value"
            >The bots don't like this slug.</span
          >
          <span class="mt-1 text-sm text-gray-500" v-else>Great choice.</span>
        </FadeTransition>
      </div>

      <!-- Description -->
      <div class="flex w-96 flex-col text-left">
        <span class="text-md text-gray-700">About</span>
        <textarea
          ref="descriptionRef"
          type="text"
          minlength="3"
          maxlength="128"
          :placeholder="isOrganization ? 'The best multinational conglomerate.' : 'The best user.'"
          :model-value="description ?? ''"
          @input="description = ($event.target as any)?.value"
          class="mt-1 w-full resize-none rounded-sm border border-orange-900 border-opacity-[15%] py-1 placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-100 focus:outline-none focus:ring-0"
          spellcheck="false"
        />
      </div>
    </div>
  </div>
</template>
