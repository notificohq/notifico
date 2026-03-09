<script lang="ts">
  import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type { Template } from '$lib/api/types';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Badge } from '$lib/components/ui/badge';
  import * as Table from '$lib/components/ui/table';
  import * as Dialog from '$lib/components/ui/dialog';

  const queryClient = useQueryClient();

  const templates = createQuery(() => ({
    queryKey: ['templates'],
    queryFn: () => api.get<Template[]>('/admin/api/v1/templates'),
  }));

  let createOpen = $state(false);
  let newName = $state('');
  let newChannel = $state('');

  const createTemplate = createMutation({
    mutationFn: (data: { name: string; channel: string }) =>
      api.post<Template>('/admin/api/v1/templates', data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['templates'] });
      createOpen = false;
      newName = '';
      newChannel = '';
    },
  });

  function handleCreate() {
    if (!newName.trim() || !newChannel.trim()) return;
    $createTemplate.mutate({ name: newName.trim(), channel: newChannel.trim() });
  }
</script>

<div class="p-8 space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-2xl font-semibold tracking-tight">Templates</h1>
      <p class="text-sm text-muted-foreground mt-1">Manage notification templates</p>
    </div>
    <Dialog.Root bind:open={createOpen}>
      <Dialog.Trigger>
        {#snippet children({ props })}
          <Button {...props}>Create Template</Button>
        {/snippet}
      </Dialog.Trigger>
      <Dialog.Content>
        <Dialog.Header>
          <Dialog.Title>Create Template</Dialog.Title>
          <Dialog.Description>Define a new notification template</Dialog.Description>
        </Dialog.Header>
        <div class="space-y-4 py-4">
          <div class="space-y-2">
            <Label>Name</Label>
            <Input bind:value={newName} placeholder="welcome-email" />
          </div>
          <div class="space-y-2">
            <Label>Channel</Label>
            <Input bind:value={newChannel} placeholder="email" class="font-mono" />
          </div>
        </div>
        <Dialog.Footer>
          <Button variant="outline" onclick={() => (createOpen = false)}>Cancel</Button>
          <Button onclick={handleCreate} disabled={$createTemplate.isPending}>
            {$createTemplate.isPending ? 'Creating...' : 'Create'}
          </Button>
        </Dialog.Footer>
      </Dialog.Content>
    </Dialog.Root>
  </div>

  {#if $templates.isLoading}
    <p class="text-sm text-muted-foreground py-8 text-center">Loading...</p>
  {:else if $templates.isError}
    <p class="text-sm text-destructive py-8 text-center">Failed to load templates</p>
  {:else if $templates.data}
    <Table.Root>
      <Table.Header>
        <Table.Row>
          <Table.Head>Name</Table.Head>
          <Table.Head>Channel</Table.Head>
          <Table.Head class="text-right">Actions</Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#each $templates.data as template}
          <Table.Row class="cursor-pointer" onclick={() => goto(`/templates/${template.id}`)}>
            <Table.Cell class="font-medium">{template.name}</Table.Cell>
            <Table.Cell>
              <Badge variant="secondary">{template.channel}</Badge>
            </Table.Cell>
            <Table.Cell class="text-right">
              <Button variant="ghost" size="sm" onclick={(e: MouseEvent) => { e.stopPropagation(); goto(`/templates/${template.id}`); }}>
                Edit
              </Button>
            </Table.Cell>
          </Table.Row>
        {/each}
        {#if $templates.data.length === 0}
          <Table.Row>
            <Table.Cell colspan={3} class="text-center text-muted-foreground py-8">
              No templates yet. Create one to get started.
            </Table.Cell>
          </Table.Row>
        {/if}
      </Table.Body>
    </Table.Root>
  {/if}
</div>
