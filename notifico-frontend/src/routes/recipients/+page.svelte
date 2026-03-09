<script lang="ts">
  import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type { Recipient } from '$lib/api/types';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Badge } from '$lib/components/ui/badge';
  import * as Table from '$lib/components/ui/table';
  import * as Dialog from '$lib/components/ui/dialog';

  const queryClient = useQueryClient();

  const recipients = createQuery(() => ({
    queryKey: ['recipients'],
    queryFn: () => api.get<Recipient[]>('/admin/api/v1/recipients'),
  }));

  let createOpen = $state(false);
  let newExternalId = $state('');
  let newLocale = $state('en');
  let newTimezone = $state('');

  const createRecipient = createMutation({
    mutationFn: (data: { external_id: string; locale: string; timezone?: string }) =>
      api.post<Recipient>('/admin/api/v1/recipients', data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['recipients'] });
      createOpen = false;
      newExternalId = '';
      newLocale = 'en';
      newTimezone = '';
    },
  });

  function handleCreate() {
    if (!newExternalId.trim()) return;
    const data: { external_id: string; locale: string; timezone?: string } = {
      external_id: newExternalId.trim(),
      locale: newLocale.trim() || 'en',
    };
    if (newTimezone.trim()) data.timezone = newTimezone.trim();
    $createRecipient.mutate(data);
  }
</script>

<div class="p-8 space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-2xl font-semibold tracking-tight">Recipients</h1>
      <p class="text-sm text-muted-foreground mt-1">Manage notification recipients</p>
    </div>
    <Dialog.Root bind:open={createOpen}>
      <Dialog.Trigger>
        {#snippet children({ props })}
          <Button {...props}>Create Recipient</Button>
        {/snippet}
      </Dialog.Trigger>
      <Dialog.Content>
        <Dialog.Header>
          <Dialog.Title>Create Recipient</Dialog.Title>
          <Dialog.Description>Register a new notification recipient</Dialog.Description>
        </Dialog.Header>
        <div class="space-y-4 py-4">
          <div class="space-y-2">
            <Label>External ID</Label>
            <Input bind:value={newExternalId} placeholder="user-123" class="font-mono" />
          </div>
          <div class="space-y-2">
            <Label>Locale</Label>
            <Input bind:value={newLocale} placeholder="en" class="font-mono" />
          </div>
          <div class="space-y-2">
            <Label>Timezone (optional)</Label>
            <Input bind:value={newTimezone} placeholder="America/New_York" class="font-mono" />
          </div>
        </div>
        <Dialog.Footer>
          <Button variant="outline" onclick={() => (createOpen = false)}>Cancel</Button>
          <Button onclick={handleCreate} disabled={$createRecipient.isPending}>
            {$createRecipient.isPending ? 'Creating...' : 'Create'}
          </Button>
        </Dialog.Footer>
      </Dialog.Content>
    </Dialog.Root>
  </div>

  {#if $recipients.isLoading}
    <p class="text-sm text-muted-foreground py-8 text-center">Loading...</p>
  {:else if $recipients.isError}
    <p class="text-sm text-destructive py-8 text-center">Failed to load recipients</p>
  {:else if $recipients.data}
    <Table.Root>
      <Table.Header>
        <Table.Row>
          <Table.Head>External ID</Table.Head>
          <Table.Head>Locale</Table.Head>
          <Table.Head class="text-right">Actions</Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#each $recipients.data as recipient}
          <Table.Row class="cursor-pointer" onclick={() => goto(`/recipients/${recipient.id}`)}>
            <Table.Cell class="font-mono text-sm">{recipient.external_id}</Table.Cell>
            <Table.Cell>
              <Badge variant="outline">{recipient.locale}</Badge>
            </Table.Cell>
            <Table.Cell class="text-right">
              <Button variant="ghost" size="sm" onclick={(e: MouseEvent) => { e.stopPropagation(); goto(`/recipients/${recipient.id}`); }}>
                View
              </Button>
            </Table.Cell>
          </Table.Row>
        {/each}
        {#if $recipients.data.length === 0}
          <Table.Row>
            <Table.Cell colspan={3} class="text-center text-muted-foreground py-8">
              No recipients yet. Create one to get started.
            </Table.Cell>
          </Table.Row>
        {/if}
      </Table.Body>
    </Table.Root>
  {/if}
</div>
