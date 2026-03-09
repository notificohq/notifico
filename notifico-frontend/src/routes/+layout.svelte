<script lang="ts">
  import '../app.css';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { isAuthenticated } from '$lib/api/client';
  import AppSidebar from '$lib/components/AppSidebar.svelte';

  let { children } = $props();

  let isLoginPage = $derived($page.url.pathname === '/login');

  $effect(() => {
    if (!isLoginPage && !isAuthenticated()) {
      goto('/login');
    }
  });
</script>

{#if isLoginPage}
  {@render children()}
{:else}
  <div class="flex min-h-screen">
    <AppSidebar />
    <main class="flex-1 overflow-y-auto lg:ml-60">
      {@render children()}
    </main>
  </div>
{/if}
