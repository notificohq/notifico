<script lang="ts">
  import { page } from '$app/stores';
  import { createQuery, createMutation } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import type { Template } from '$lib/api/types';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Badge } from '$lib/components/ui/badge';
  import { Textarea } from '$lib/components/ui/textarea';
  import * as Card from '$lib/components/ui/card';
  import * as Tabs from '$lib/components/ui/tabs';

  let templateId = $derived($page.params.id);

  const template = createQuery(() => ({
    queryKey: ['templates', templateId],
    queryFn: () => api.get<Template>(`/admin/api/v1/templates/${templateId}`),
  }));

  // Locale management
  let locales = $state<string[]>(['en']);
  let activeLocale = $state('en');
  let newLocale = $state('');

  // Content per locale
  let contentMap = $state<Record<string, string>>({ en: '{\n  "subject": "",\n  "html": ""\n}' });
  let saveStatus = $state<Record<string, string>>({});

  // Preview
  let previewData = $state('{}');
  let previewResult = $state<string | null>(null);
  let previewError = $state<string | null>(null);

  // Load content for active locale
  $effect(() => {
    if (templateId && activeLocale) {
      api.get<{ body: unknown }>(`/admin/api/v1/templates/${templateId}/content/${activeLocale}`)
        .then((data) => {
          contentMap[activeLocale] = JSON.stringify(data.body, null, 2);
        })
        .catch(() => {
          // Content may not exist yet for this locale
          if (!contentMap[activeLocale]) {
            contentMap[activeLocale] = '{\n  "subject": "",\n  "html": ""\n}';
          }
        });
    }
  });

  const saveContent = createMutation({
    mutationFn: (data: { locale: string; body: unknown }) =>
      api.put(`/admin/api/v1/templates/${templateId}/content/${data.locale}`, { body: data.body }),
    onSuccess: (_data, variables) => {
      saveStatus[variables.locale] = 'Saved';
      setTimeout(() => { saveStatus[variables.locale] = ''; }, 2000);
    },
    onError: (_error, variables) => {
      saveStatus[variables.locale] = 'Error saving';
    },
  });

  function handleSave() {
    try {
      const body = JSON.parse(contentMap[activeLocale]);
      $saveContent.mutate({ locale: activeLocale, body });
    } catch {
      saveStatus[activeLocale] = 'Invalid JSON';
    }
  }

  function addLocale() {
    const loc = newLocale.trim().toLowerCase();
    if (loc && !locales.includes(loc)) {
      locales = [...locales, loc];
      contentMap[loc] = '{\n  "subject": "",\n  "html": ""\n}';
      activeLocale = loc;
      newLocale = '';
    }
  }

  async function handlePreview() {
    previewError = null;
    previewResult = null;
    try {
      const data = JSON.parse(previewData);
      const result = await api.post<{ rendered: string }>(`/admin/api/v1/templates/${templateId}/preview`, {
        locale: activeLocale,
        data,
      });
      previewResult = typeof result.rendered === 'string' ? result.rendered : JSON.stringify(result, null, 2);
    } catch (e) {
      previewError = e instanceof Error ? e.message : 'Preview failed';
    }
  }
</script>

<div class="p-8 space-y-8">
  <!-- Header -->
  <div>
    <a href="/templates" class="text-sm text-muted-foreground hover:text-foreground transition-colors">&larr; Back to Templates</a>
    {#if $template.data}
      <h1 class="text-2xl font-semibold tracking-tight mt-2">{$template.data.name}</h1>
      <Badge variant="secondary" class="mt-1">{$template.data.channel}</Badge>
    {:else}
      <h1 class="text-2xl font-semibold tracking-tight mt-2">Loading...</h1>
    {/if}
  </div>

  <!-- Content Editor -->
  <Card.Root>
    <Card.Header>
      <Card.Title>Template Content</Card.Title>
      <Card.Description>Edit template content per locale</Card.Description>
    </Card.Header>
    <Card.Content class="space-y-4">
      <!-- Locale tabs + add -->
      <div class="flex items-center gap-2">
        <Tabs.Root value={activeLocale} onValueChange={(v) => { if (v) activeLocale = v; }}>
          <Tabs.List>
            {#each locales as locale}
              <Tabs.Trigger value={locale}>{locale}</Tabs.Trigger>
            {/each}
          </Tabs.List>
        </Tabs.Root>
        <div class="flex items-center gap-1 ml-4">
          <Input bind:value={newLocale} placeholder="fr" class="w-16 h-8 text-sm font-mono" />
          <Button variant="outline" size="sm" onclick={addLocale}>+</Button>
        </div>
      </div>

      <!-- Editor -->
      <div class="space-y-2">
        <Label>Body (JSON)</Label>
        <Textarea
          value={contentMap[activeLocale] ?? ''}
          oninput={(e: globalThis.Event & { currentTarget: HTMLTextAreaElement }) => { contentMap[activeLocale] = e.currentTarget.value; }}
          class="font-mono min-h-[250px] text-sm"
          placeholder={'{"subject": "Hello", "html": "<h1>Welcome</h1>"}'}
        />
      </div>

      <div class="flex items-center gap-3">
        <Button onclick={handleSave} disabled={$saveContent.isPending}>
          {$saveContent.isPending ? 'Saving...' : 'Save'}
        </Button>
        {#if saveStatus[activeLocale]}
          <span class="text-sm text-muted-foreground">{saveStatus[activeLocale]}</span>
        {/if}
      </div>
    </Card.Content>
  </Card.Root>

  <!-- Preview -->
  <Card.Root>
    <Card.Header>
      <Card.Title>Preview</Card.Title>
      <Card.Description>Test template rendering with sample data</Card.Description>
    </Card.Header>
    <Card.Content class="space-y-4">
      <div class="space-y-2">
        <Label>Template Data (JSON)</Label>
        <Textarea
          bind:value={previewData}
          class="font-mono min-h-[100px] text-sm"
          placeholder={'{"name": "John", "order_id": "12345"}'}
        />
      </div>
      <Button variant="secondary" onclick={handlePreview}>Preview</Button>

      {#if previewError}
        <div class="rounded border border-destructive/50 bg-destructive/10 p-4 text-sm text-destructive">
          {previewError}
        </div>
      {/if}

      {#if previewResult}
        <div class="rounded border border-border bg-muted/30 p-4">
          <pre class="text-sm font-mono whitespace-pre-wrap">{previewResult}</pre>
        </div>
      {/if}
    </Card.Content>
  </Card.Root>
</div>
