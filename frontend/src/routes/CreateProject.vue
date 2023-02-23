<script lang="ts" setup>
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import { useTitle } from "@vueuse/core";
import { computed, ref, type Ref } from "vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useOperations } from "@/state/operations";
import { useAuth } from "@/state/auth";
import { ProjectType, ProjectVisibility } from "@/gql/graphql";
import { useRouter } from "vue-router";
import { useNotifications } from "@/state/notifications";
import NotificationArea from "@/components/container/NotificationArea.vue";
import { useQuery } from "@vue/apollo-composable";
import { graphql } from "@/gql";

const title = useTitle();
title.value = "Bench - Create bench";

const auth = useAuth();
const owner: Ref<{ id: string; slug: string }> = computed(() => auth.me.value);
const name: Ref<string> = ref("Sandbox");
const slug: Ref<string> = ref("sandbox");
const slugModified = ref(false);
const isPublic: Ref<boolean> = ref(true);
const type: Ref<ProjectType> = ref(ProjectType.Executable);

function syncSlugIfUnmodified() {
  if (!slugModified.value) {
    slug.value = name.value.toLowerCase().replace(/[^a-z0-9_-]/g, "-");
  }
}

const isValidName = computed(() => (name.value?.length ?? 0) >= 2);
const isValidSlug = computed(() => /^[a-z0-9_-]{3,}$/.test(slug.value ?? "") && (slug.value?.length ?? 0 >= 4));

const { result: existingProject, loading: existingProjectLoading } = useQuery(
  graphql(/* GraphQL */ `
    query existingProjectBySlug($owner: String!, $project: String!) {
      projectBySlug(owner: $owner, project: $project) {
        id
        slug
      }
    }
  `),
  computed(() => ({
    owner: owner.value?.slug,
    project: slug.value,
  }))
);
const isAvailableSlug = computed(() => existingProject.value?.projectBySlug == null);
const canComplete = computed(
  () =>
    !creating.value && !existingProjectLoading.value && isValidName.value && isValidSlug.value && isAvailableSlug.value
);

const creating: Ref<boolean> = ref(false);

const ops = useOperations();
const router = useRouter();
const notifications = useNotifications();
async function createProject() {
  creating.value = true;
  const create = await ops.project.create(
    owner.value.id,
    name.value,
    slug.value,
    type.value,
    isPublic.value ? ProjectVisibility.Private : ProjectVisibility.Private
  );
  creating.value = false;
  if (create?.data?.createProject.__typename == "Project") {
    router.push(`/${owner.value.slug}/${slug.value}`);
    notifications.show({
      kind: "success",
      type: "project.created",
      message: "Bench born",
      description: "Your bench has been created",
    });
  }
}
</script>
<template>
  <div class="flex h-full flex-col bg-white pb-12">
    <FatHeader>
      <template v-slot:left>
        <HomeButton />
      </template>
      <template v-slot:right>
        <ProfileButton />
      </template>
    </FatHeader>
    <div class="mx-auto mt-20 w-72 text-center lg:mt-32">
      <div class="flex flex-row items-baseline justify-center gap-1">
        <div class="font-mono text-2xl font-bold">
          <span class="-mx-0.5 text-gray-900">[</span>
          <span class="text-3xl text-orange-600">x</span>
          <span class="-mx-0.5 text-gray-900">]</span>
        </div>
        <h3 class="font-mono text-2xl font-bold">Bench</h3>
      </div>
      <h1 class="mt-4 text-5xl font-bold">Create bench</h1>
      <!-- Fields to complete -->
      <div class="mt-10 flex flex-col gap-4">
        <!-- TODO @Incomplete: select owner -->

        <!-- Full name -->
        <div class="text-left">
          <span class="text-md text-gray-700">Bench name</span>
          <input
            type="text"
            minlength="3"
            maxlength="128"
            v-model="name"
            @input="syncSlugIfUnmodified"
            class="mt-1 w-full rounded-sm border border-orange-600 py-1 placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-50 focus:outline-none focus:ring-0"
          />
          <FadeTransition mode="out-in">
            <span class="mt-1 text-sm text-yellow-500" v-if="!isValidName">That's not a name we can print.</span>
            <span class="mt-1 text-sm text-gray-500" v-else>Great name.</span>
          </FadeTransition>
        </div>

        <!-- slug -->
        <div class="text-left">
          <span class="text-md text-gray-700">Pick a short name</span>
          <input
            type="text"
            minlength="3"
            maxlength="128"
            pattern="[a-z0-9_-]+"
            :value="slug"
            @input="(event) => ((slug = event.target?.value), (slugModified = true))"
            class="mt-1 w-full rounded-sm border border-orange-600 py-1 placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-50 focus:outline-none focus:ring-0"
          />
          <FadeTransition mode="out-in">
            <span v-if="!isValidSlug" class="mt-1 text-sm text-orange-600">
              Invalid short name. <span class="font-mono text-xs text-gray-500">[a-z0-9_-]{3,}</span>
            </span>
            <span v-else-if="existingProjectLoading" class="mt-1">&nbsp;</span>
            <span v-else-if="!isAvailableSlug" class="mt-1 text-sm text-red-600">
              That name is
              <router-link
                :to="`/${owner.slug}/${slug}`"
                class="underline decoration-dotted underline-offset-2 hover:decoration-solid focus:decoration-solid focus:outline-none"
                >taken</router-link
              >.
            </span>
            <span v-else class="mt-1 text-sm text-gray-500">Yours for the taking.</span>
          </FadeTransition>
        </div>

        <button
          :disabled="!canComplete"
          class="mt-6 w-fit self-end border border-orange-600 px-3 py-1 hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
          :class="{ 'pointer-events-none opacity-50': !canComplete }"
          @click="createProject"
        >
          Build &rarr;
        </button>
      </div>
    </div>
    <NotificationArea />
  </div>
</template>
