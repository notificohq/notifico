<script lang="ts">
  import { goto } from '$app/navigation';
  import { setApiKey } from '$lib/api/client';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';

  let apiKey = $state('');
  let error = $state('');
  let loading = $state(false);
  let showKey = $state(false);
  let mounted = $state(false);

  $effect(() => {
    mounted = true;
  });

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (!apiKey.trim()) {
      error = 'Please enter an API key';
      return;
    }

    loading = true;
    error = '';

    try {
      const resp = await fetch('/admin/api/v1/projects', {
        headers: {
          'Authorization': `Bearer ${apiKey.trim()}`,
          'Content-Type': 'application/json',
        },
      });

      if (resp.ok) {
        setApiKey(apiKey.trim());
        goto('/dashboard');
      } else if (resp.status === 401) {
        error = 'Invalid API key';
      } else {
        error = `Connection failed (${resp.status})`;
      }
    } catch (err) {
      error = 'Unable to connect to server';
    } finally {
      loading = false;
    }
  }
</script>

<div
  class="flex min-h-screen items-center justify-center transition-opacity duration-700"
  class:opacity-0={!mounted}
  class:opacity-100={mounted}
  style="background: radial-gradient(ellipse at 50% 30%, oklch(0.16 0.01 260) 0%, oklch(0.10 0.008 260) 70%);"
>
  <div class="w-full max-w-[400px] px-6">
    <div class="rounded-lg border border-border bg-card p-8">
      <div class="mb-8 text-center">
        <h1
          class="text-2xl font-bold tracking-[0.2em] text-foreground"
          style="font-family: 'JetBrains Mono', monospace;"
        >
          NOTIFICO
        </h1>
        <p class="mt-2 text-sm text-muted-foreground">Admin Panel</p>
      </div>

      <form onsubmit={handleSubmit} class="space-y-4">
        <div class="space-y-2">
          <label for="api-key" class="text-sm font-medium text-muted-foreground">API Key</label>
          <div class="relative">
            <Input
              id="api-key"
              type={showKey ? 'text' : 'password'}
              placeholder="nk_..."
              bind:value={apiKey}
              class="pr-10 font-mono"
              style="font-family: 'JetBrains Mono', monospace;"
            />
            <button
              type="button"
              onclick={() => (showKey = !showKey)}
              class="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground"
            >
              {#if showKey}
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/><path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
              {:else}
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
              {/if}
            </button>
          </div>
        </div>

        {#if error}
          <p class="text-sm text-destructive">{error}</p>
        {/if}

        <Button
          type="submit"
          class="w-full bg-primary text-primary-foreground hover:bg-primary/90"
          disabled={loading}
        >
          {loading ? 'Connecting...' : 'Connect'}
        </Button>
      </form>
    </div>
  </div>
</div>
