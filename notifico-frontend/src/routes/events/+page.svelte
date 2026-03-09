<script lang="ts">
  import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type { Event } from '$lib/api/types';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Badge } from '$lib/components/ui/badge';
  import * as Table from '$lib/components/ui/table';
  import * as Dialog from '$lib/components/ui/dialog';
  import * as Select from '$lib/components/ui/select';

  const queryClient = useQueryClient();

  const events = createQuery(() => ({
    queryKey: ['events'],
    queryFn: () => api.get<Event[]>('/admin/api/v1/events'),
  }));

  let createOpen = $state(false);
  let newName = $state('');
  let newCategory = $state('transactional');

  let deleteTarget = $state<Event | null>(null);

  const createEvent = createMutation({
    mutationFn: (data: { name: string; category: string }) =>
      api.post<Event>('/admin/api/v1/events', data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['events'] });
      createOpen = false;
      newName = '';
      newCategory = 'transactional';
    },
  });

  const deleteEvent = createMutation({
    mutationFn: (id: string) => api.delete(`/admin/api/v1/events/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['events'] });
      deleteTarget = null;
    },
  });

  function handleCreate() {
    if (!newName.trim()) return;
    $createEvent.mutate({ name: newName.trim(), category: newCategory });
  }

  function categoryVariant(cat: string): 'default' | 'secondary' | 'outline' {
    switch (cat) {
      case 'transactional': return 'default';
      case 'marketing': return 'secondary';
      case 'system': return 'outline';
      default: return 'secondary';
    }
  }
</script>

<div class="p-8 space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-2xl font-semibold tracking-tight">Events</h1>
      <p class="text-sm text-muted-foreground mt-1">Manage notification events and their pipeline rules</p>
    </div>
    <Dialog.Root bind:open={createOpen}>
      <Dialog.Trigger>
        {#snippet children({ props })}
          <Button {...props}>Create Event</Button>
        {/snippet}
      </Dialog.Trigger>
      <Dialog.Content>
        <Dialog.Header>
          <Dialog.Title>Create Event</Dialog.Title>
          <Dialog.Description>Define a new notification event</Dialog.Description>
        </Dialog.Header>
        <div class="space-y-4 py-4">
          <div class="space-y-2">
            <Label>Name</Label>
            <Input bind:value={newName} placeholder="order.confirmed" class="font-mono" />
          </div>
          <div class="space-y-2">
            <Label>Category</Label>
            <Select.Root type="single" value={newCategory} onValueChange={(v) => { if (v) newCategory = v; }}>
              <Select.Trigger class="w-full">
                <span>{newCategory}</span>
              </Select.Trigger>
              <Select.Content>
                <Select.Item value="transactional">transactional</Select.Item>
                <Select.Item value="marketing">marketing</Select.Item>
                <Select.Item value="system">system</Select.Item>
              </Select.Content>
            </Select.Root>
          </div>
        </div>
        <Dialog.Footer>
          <Button variant="outline" onclick={() => (createOpen = false)}>Cancel</Button>
          <Button onclick={handleCreate} disabled={$createEvent.isPending}>
            {$createEvent.isPending ? 'Creating...' : 'Create'}
          </Button>
        </Dialog.Footer>
      </Dialog.Content>
    </Dialog.Root>
  </div>

  {#if $events.isLoading}
    <p class="text-sm text-muted-foreground py-8 text-center">Loading...</p>
  {:else if $events.isError}
    <p class="text-sm text-destructive py-8 text-center">Failed to load events</p>
  {:else if $events.data}
    <Table.Root>
      <Table.Header>
        <Table.Row>
          <Table.Head>Name</Table.Head>
          <Table.Head>Category</Table.Head>
          <Table.Head class="text-right">Actions</Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#each $events.data as event}
          <Table.Row class="cursor-pointer" onclick={() => goto(`/events/${event.id}`)}>
            <Table.Cell class="font-mono text-sm">{event.name}</Table.Cell>
            <Table.Cell>
              <Badge variant={categoryVariant(event.category)}>{event.category}</Badge>
            </Table.Cell>
            <Table.Cell class="text-right">
              <Button variant="ghost" size="sm" onclick={(e: MouseEvent) => { e.stopPropagation(); goto(`/events/${event.id}`); }}>
                View
              </Button>
              <Button variant="ghost" size="sm" class="text-destructive" onclick={(e: MouseEvent) => { e.stopPropagation(); deleteTarget = event; }}>
                Delete
              </Button>
            </Table.Cell>
          </Table.Row>
        {/each}
        {#if $events.data.length === 0}
          <Table.Row>
            <Table.Cell colspan={3} class="text-center text-muted-foreground py-8">
              No events yet. Create one to get started.
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
      <Dialog.Title>Delete Event</Dialog.Title>
      <Dialog.Description>
        Are you sure you want to delete <span class="font-mono">{deleteTarget?.name}</span>? This action cannot be undone.
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (deleteTarget = null)}>Cancel</Button>
      <Button variant="destructive" onclick={() => { if (deleteTarget) $deleteEvent.mutate(deleteTarget.id); }} disabled={$deleteEvent.isPending}>
        {$deleteEvent.isPending ? 'Deleting...' : 'Delete'}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
