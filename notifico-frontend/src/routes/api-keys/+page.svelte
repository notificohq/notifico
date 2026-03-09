<script lang="ts">
  import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import type { ApiKey } from '$lib/api/types';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Badge } from '$lib/components/ui/badge';
  import * as Table from '$lib/components/ui/table';
  import * as Dialog from '$lib/components/ui/dialog';
  import * as Select from '$lib/components/ui/select';
  import * as Card from '$lib/components/ui/card';

  const queryClient = useQueryClient();

  const apiKeys = createQuery(() => ({
    queryKey: ['api-keys'],
    queryFn: () => api.get<ApiKey[]>('/admin/api/v1/api-keys'),
  }));

  let createOpen = $state(false);
  let newName = $state('');
  let newScope = $state('ingest');

  let createdKey = $state<ApiKey | null>(null);
  let deleteTarget = $state<ApiKey | null>(null);

  const createApiKey = createMutation({
    mutationFn: (data: { name: string; scope: string }) =>
      api.post<ApiKey>('/admin/api/v1/api-keys', data),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['api-keys'] });
      createOpen = false;
      newName = '';
      newScope = 'ingest';
      createdKey = data;
    },
  });

  const deleteApiKey = createMutation({
    mutationFn: (id: string) => api.delete(`/admin/api/v1/api-keys/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['api-keys'] });
      deleteTarget = null;
    },
  });

  function handleCreate() {
    if (!newName.trim()) return;
    $createApiKey.mutate({ name: newName.trim(), scope: newScope });
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleString();
  }

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
  }
</script>

<div class="p-8 space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-2xl font-semibold tracking-tight">API Keys</h1>
      <p class="text-sm text-muted-foreground mt-1">Manage API access keys</p>
    </div>
    <Dialog.Root bind:open={createOpen}>
      <Dialog.Trigger>
        {#snippet children({ props })}
          <Button {...props}>Create API Key</Button>
        {/snippet}
      </Dialog.Trigger>
      <Dialog.Content>
        <Dialog.Header>
          <Dialog.Title>Create API Key</Dialog.Title>
          <Dialog.Description>Generate a new API key for programmatic access</Dialog.Description>
        </Dialog.Header>
        <div class="space-y-4 py-4">
          <div class="space-y-2">
            <Label>Name</Label>
            <Input bind:value={newName} placeholder="production-ingest" />
          </div>
          <div class="space-y-2">
            <Label>Scope</Label>
            <Select.Root type="single" value={newScope} onValueChange={(v) => { if (v) newScope = v; }}>
              <Select.Trigger class="w-full">
                <span>{newScope}</span>
              </Select.Trigger>
              <Select.Content>
                <Select.Item value="ingest">ingest</Select.Item>
                <Select.Item value="admin">admin</Select.Item>
              </Select.Content>
            </Select.Root>
          </div>
        </div>
        <Dialog.Footer>
          <Button variant="outline" onclick={() => (createOpen = false)}>Cancel</Button>
          <Button onclick={handleCreate} disabled={$createApiKey.isPending}>
            {$createApiKey.isPending ? 'Creating...' : 'Create'}
          </Button>
        </Dialog.Footer>
      </Dialog.Content>
    </Dialog.Root>
  </div>

  {#if $apiKeys.isLoading}
    <p class="text-sm text-muted-foreground py-8 text-center">Loading...</p>
  {:else if $apiKeys.isError}
    <p class="text-sm text-destructive py-8 text-center">Failed to load API keys</p>
  {:else if $apiKeys.data}
    <Table.Root>
      <Table.Header>
        <Table.Row>
          <Table.Head>Name</Table.Head>
          <Table.Head>Scope</Table.Head>
          <Table.Head>Prefix</Table.Head>
          <Table.Head>Created</Table.Head>
          <Table.Head class="text-right">Actions</Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#each $apiKeys.data as key}
          <Table.Row>
            <Table.Cell class="font-medium">{key.name}</Table.Cell>
            <Table.Cell>
              <Badge variant={key.scope === 'admin' ? 'default' : 'secondary'}>{key.scope}</Badge>
            </Table.Cell>
            <Table.Cell class="font-mono text-sm text-muted-foreground">{key.prefix}...</Table.Cell>
            <Table.Cell class="text-sm text-muted-foreground">{formatDate(key.created_at)}</Table.Cell>
            <Table.Cell class="text-right">
              <Button variant="ghost" size="sm" class="text-destructive" onclick={() => (deleteTarget = key)}>
                Delete
              </Button>
            </Table.Cell>
          </Table.Row>
        {/each}
        {#if $apiKeys.data.length === 0}
          <Table.Row>
            <Table.Cell colspan={5} class="text-center text-muted-foreground py-8">
              No API keys yet. Create one to get started.
            </Table.Cell>
          </Table.Row>
        {/if}
      </Table.Body>
    </Table.Root>
  {/if}
</div>

<!-- Created Key Display -->
<Dialog.Root open={!!createdKey} onOpenChange={(open) => { if (!open) createdKey = null; }}>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>API Key Created</Dialog.Title>
      <Dialog.Description>
        Copy this key now. You will not be able to see it again.
      </Dialog.Description>
    </Dialog.Header>
    {#if createdKey?.raw_key}
      <Card.Root class="bg-muted/50">
        <Card.Content class="pt-6">
          <div class="flex items-center gap-2">
            <code class="flex-1 break-all text-sm font-mono">{createdKey.raw_key}</code>
            <Button variant="outline" size="sm" onclick={() => copyToClipboard(createdKey!.raw_key!)}>
              Copy
            </Button>
          </div>
        </Card.Content>
      </Card.Root>
    {/if}
    <Dialog.Footer>
      <Button onclick={() => (createdKey = null)}>Done</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<!-- Delete Confirmation -->
<Dialog.Root open={!!deleteTarget} onOpenChange={(open) => { if (!open) deleteTarget = null; }}>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>Delete API Key</Dialog.Title>
      <Dialog.Description>
        Are you sure you want to delete <span class="font-semibold">{deleteTarget?.name}</span>? This action cannot be undone.
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (deleteTarget = null)}>Cancel</Button>
      <Button variant="destructive" onclick={() => { if (deleteTarget) $deleteApiKey.mutate(deleteTarget.id); }} disabled={$deleteApiKey.isPending}>
        {$deleteApiKey.isPending ? 'Deleting...' : 'Delete'}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
