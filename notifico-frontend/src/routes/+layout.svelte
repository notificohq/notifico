<script lang="ts">
  import '../app.css';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { isAuthenticated } from '$lib/api/client';
  import AppSidebar from '$lib/components/AppSidebar.svelte';
  import { QueryClient, QueryClientProvider } from '@tanstack/svelte-query';

  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { staleTime: 30000, refetchOnWindowFocus: false },
    },
  });

  let { children } = $props();

  let isLoginPage = $derived($page.url.pathname === '/login');

  $effect(() => {
    if (!isLoginPage && !isAuthenticated()) {
      goto('/login');
    }
  });
</script>

<QueryClientProvider client={queryClient}>
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
</QueryClientProvider>
