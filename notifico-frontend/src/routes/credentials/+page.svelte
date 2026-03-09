<script lang="ts">
  import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import type { Credential } from '$lib/api/types';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Badge } from '$lib/components/ui/badge';
  import { Textarea } from '$lib/components/ui/textarea';
  import * as Table from '$lib/components/ui/table';
  import * as Dialog from '$lib/components/ui/dialog';

  const queryClient = useQueryClient();

  const credentials = createQuery(() => ({
    queryKey: ['credentials'],
    queryFn: () => api.get<Credential[]>('/admin/api/v1/credentials'),
  }));

  let createOpen = $state(false);
  let newName = $state('');
  let newChannel = $state('');
  let newConfig = $state('{}');

  let deleteTarget = $state<Credential | null>(null);

  const createCredential = createMutation({
    mutationFn: (payload: { name: string; channel: string; data: unknown }) =>
      api.post<Credential>('/admin/api/v1/credentials', payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['credentials'] });
      createOpen = false;
      newName = '';
      newChannel = '';
      newConfig = '{}';
    },
  });

  const deleteCredential = createMutation({
    mutationFn: (id: string) => api.delete(`/admin/api/v1/credentials/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['credentials'] });
      deleteTarget = null;
    },
  });

  let configError = $state('');

  function handleCreate() {
    if (!newName.trim() || !newChannel.trim()) return;
    let parsed: unknown;
    try {
      parsed = JSON.parse(newConfig);
      configError = '';
    } catch {
      configError = 'Invalid JSON';
      return;
    }
    $createCredential.mutate({
      name: newName.trim(),
      channel: newChannel.trim(),
      data: parsed,
    });
  }
</script>

<div class="p-8 space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-2xl font-semibold tracking-tight">Credentials</h1>
      <p class="text-sm text-muted-foreground mt-1">Manage channel delivery credentials</p>
    </div>
    <Dialog.Root bind:open={createOpen}>
      <Dialog.Trigger>
        {#snippet children({ props })}
          <Button {...props}>Create Credential</Button>
        {/snippet}
      </Dialog.Trigger>
      <Dialog.Content>
        <Dialog.Header>
          <Dialog.Title>Create Credential</Dialog.Title>
          <Dialog.Description>Add credentials for a delivery channel</Dialog.Description>
        </Dialog.Header>
        <div class="space-y-4 py-4">
          <div class="space-y-2">
            <Label>Name</Label>
            <Input bind:value={newName} placeholder="sendgrid-production" />
          </div>
          <div class="space-y-2">
            <Label>Channel</Label>
            <Input bind:value={newChannel} placeholder="email" class="font-mono" />
          </div>
          <div class="space-y-2">
            <Label>Config (JSON)</Label>
            <Textarea bind:value={newConfig} placeholder={'{}'} class="font-mono min-h-[120px]" />
            {#if configError}
              <p class="text-sm text-destructive">{configError}</p>
            {/if}
          </div>
        </div>
        <Dialog.Footer>
          <Button variant="outline" onclick={() => (createOpen = false)}>Cancel</Button>
          <Button onclick={handleCreate} disabled={$createCredential.isPending}>
            {$createCredential.isPending ? 'Creating...' : 'Create'}
          </Button>
        </Dialog.Footer>
      </Dialog.Content>
    </Dialog.Root>
  </div>

  {#if $credentials.isLoading}
    <p class="text-sm text-muted-foreground py-8 text-center">Loading...</p>
  {:else if $credentials.isError}
    <p class="text-sm text-destructive py-8 text-center">Failed to load credentials</p>
  {:else if $credentials.data}
    <Table.Root>
      <Table.Header>
        <Table.Row>
          <Table.Head>Name</Table.Head>
          <Table.Head>Channel</Table.Head>
          <Table.Head>Enabled</Table.Head>
          <Table.Head class="text-right">Actions</Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#each $credentials.data as cred}
          <Table.Row>
            <Table.Cell class="font-medium">{cred.name}</Table.Cell>
            <Table.Cell>
              <Badge variant="secondary">{cred.channel}</Badge>
            </Table.Cell>
            <Table.Cell>
              <Badge variant={cred.enabled ? 'default' : 'outline'}>
                {cred.enabled ? 'enabled' : 'disabled'}
              </Badge>
            </Table.Cell>
            <Table.Cell class="text-right">
              <Button variant="ghost" size="sm" class="text-destructive" onclick={() => (deleteTarget = cred)}>
                Delete
              </Button>
            </Table.Cell>
          </Table.Row>
        {/each}
        {#if $credentials.data.length === 0}
          <Table.Row>
            <Table.Cell colspan={4} class="text-center text-muted-foreground py-8">
              No credentials yet. Create one to enable delivery.
            </Table.Cell>
          </Table.Row>
        {/if}
      </Table.Body>
    </Table.Root>
  {/if}
</div>

<!-- Delete Confirmation -->
<Dialog.Root open={!!deleteTarget} onOpenChange={(open) => { if (!open) deleteTarget = null; }}>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>Delete Credential</Dialog.Title>
      <Dialog.Description>
        Are you sure you want to delete <span class="font-semibold">{deleteTarget?.name}</span>? This action cannot be undone.
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (deleteTarget = null)}>Cancel</Button>
      <Button variant="destructive" onclick={() => { if (deleteTarget) $deleteCredential.mutate(deleteTarget.id); }} disabled={$deleteCredential.isPending}>
        {$deleteCredential.isPending ? 'Deleting...' : 'Delete'}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
