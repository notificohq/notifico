<script lang="ts">
  import { page } from '$app/stores';
  import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import type { Recipient, Contact } from '$lib/api/types';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Badge } from '$lib/components/ui/badge';
  import * as Table from '$lib/components/ui/table';
  import * as Dialog from '$lib/components/ui/dialog';
  import * as Card from '$lib/components/ui/card';

  const queryClient = useQueryClient();
  let recipientId = $derived($page.params.id);

  const recipient = createQuery(() => ({
    queryKey: ['recipients', recipientId],
    queryFn: () => api.get<Recipient>(`/admin/api/v1/recipients/${recipientId}`),
  }));

  const contacts = createQuery(() => ({
    queryKey: ['recipients', recipientId, 'contacts'],
    queryFn: () => api.get<Contact[]>(`/admin/api/v1/recipients/${recipientId}/contacts`),
  }));

  let addContactOpen = $state(false);
  let contactChannel = $state('');
  let contactValue = $state('');

  const createContact = createMutation({
    mutationFn: (data: { channel: string; value: string }) =>
      api.post(`/admin/api/v1/recipients/${recipientId}/contacts`, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['recipients', recipientId, 'contacts'] });
      addContactOpen = false;
      contactChannel = '';
      contactValue = '';
    },
  });

  const deleteContact = createMutation({
    mutationFn: (contactId: string) => api.delete(`/admin/api/v1/contacts/${contactId}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['recipients', recipientId, 'contacts'] });
    },
  });
</script>

<div class="p-8 space-y-8">
  <!-- Header -->
  <div>
    <a href="/recipients" class="text-sm text-muted-foreground hover:text-foreground transition-colors">&larr; Back to Recipients</a>
    {#if $recipient.data}
      <h1 class="text-2xl font-semibold tracking-tight mt-2 font-mono">{$recipient.data.external_id}</h1>
      <div class="flex items-center gap-2 mt-1">
        <Badge variant="outline">{$recipient.data.locale}</Badge>
        {#if $recipient.data.timezone}
          <Badge variant="secondary">{$recipient.data.timezone}</Badge>
        {/if}
      </div>
    {:else}
      <h1 class="text-2xl font-semibold tracking-tight mt-2">Loading...</h1>
    {/if}
  </div>

  <!-- Contacts -->
  <Card.Root>
    <Card.Header>
      <div class="flex items-center justify-between">
        <Card.Title>Contacts</Card.Title>
        <Dialog.Root bind:open={addContactOpen}>
          <Dialog.Trigger>
            {#snippet children({ props })}
              <Button size="sm" {...props}>Add Contact</Button>
            {/snippet}
          </Dialog.Trigger>
          <Dialog.Content>
            <Dialog.Header>
              <Dialog.Title>Add Contact</Dialog.Title>
              <Dialog.Description>Add a delivery channel contact for this recipient</Dialog.Description>
            </Dialog.Header>
            <div class="space-y-4 py-4">
              <div class="space-y-2">
                <Label>Channel</Label>
                <Input bind:value={contactChannel} placeholder="email" class="font-mono" />
              </div>
              <div class="space-y-2">
                <Label>Value</Label>
                <Input bind:value={contactValue} placeholder="user@example.com" class="font-mono" />
              </div>
            </div>
            <Dialog.Footer>
              <Button variant="outline" onclick={() => (addContactOpen = false)}>Cancel</Button>
              <Button onclick={() => $createContact.mutate({ channel: contactChannel, value: contactValue })} disabled={$createContact.isPending}>
                {$createContact.isPending ? 'Adding...' : 'Add Contact'}
              </Button>
            </Dialog.Footer>
          </Dialog.Content>
        </Dialog.Root>
      </div>
    </Card.Header>
    <Card.Content>
      {#if $contacts.isLoading}
        <p class="text-sm text-muted-foreground py-4 text-center">Loading contacts...</p>
      {:else if $contacts.data}
        <Table.Root>
          <Table.Header>
            <Table.Row>
              <Table.Head>Channel</Table.Head>
              <Table.Head>Value</Table.Head>
              <Table.Head class="text-right">Actions</Table.Head>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {#each $contacts.data as contact}
              <Table.Row>
                <Table.Cell>
                  <Badge variant="secondary">{contact.channel}</Badge>
                </Table.Cell>
                <Table.Cell class="font-mono text-sm">{contact.value}</Table.Cell>
                <Table.Cell class="text-right">
                  <Button variant="ghost" size="sm" class="text-destructive" onclick={() => $deleteContact.mutate(contact.id)}>
                    Delete
                  </Button>
                </Table.Cell>
              </Table.Row>
            {/each}
            {#if $contacts.data.length === 0}
              <Table.Row>
                <Table.Cell colspan={3} class="text-center text-muted-foreground py-8">
                  No contacts yet. Add one to enable delivery.
                </Table.Cell>
              </Table.Row>
            {/if}
          </Table.Body>
        </Table.Root>
      {/if}
    </Card.Content>
  </Card.Root>
</div>
