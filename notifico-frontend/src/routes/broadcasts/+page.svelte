<script lang="ts">
  import { createMutation } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Textarea } from '$lib/components/ui/textarea';
  import * as Card from '$lib/components/ui/card';

  let eventName = $state('');
  let dataJson = $state('{}');
  let recipientIds = $state('');
  let showRecipients = $state(false);

  interface BroadcastResult {
    broadcast_id: string;
    recipient_count: number;
    task_count: number;
  }

  let result = $state<BroadcastResult | null>(null);
  let error = $state<string | null>(null);

  const sendBroadcast = createMutation({
    mutationFn: (payload: { event: string; data: unknown; recipients?: string[] }) =>
      api.post<BroadcastResult>('/api/v1/broadcasts', payload),
    onSuccess: (data) => {
      result = data;
      error = null;
    },
    onError: (err) => {
      error = err instanceof Error ? err.message : 'Broadcast failed';
      result = null;
    },
  });

  function handleSend() {
    if (!eventName.trim()) return;
    error = null;
    try {
      const data = JSON.parse(dataJson);
      const payload: { event: string; data: unknown; recipients?: string[] } = {
        event: eventName.trim(),
        data,
      };
      if (showRecipients && recipientIds.trim()) {
        payload.recipients = recipientIds.split(',').map((s) => s.trim()).filter(Boolean);
      }
      $sendBroadcast.mutate(payload);
    } catch {
      error = 'Invalid JSON in data field';
    }
  }
</script>

<div class="p-8 space-y-6">
  <div>
    <h1 class="text-2xl font-semibold tracking-tight">Broadcasts</h1>
    <p class="text-sm text-muted-foreground mt-1">Send notifications to multiple recipients</p>
  </div>

  <Card.Root class="max-w-2xl">
    <Card.Header>
      <Card.Title>Send Broadcast</Card.Title>
      <Card.Description>Trigger a notification event for matching recipients</Card.Description>
    </Card.Header>
    <Card.Content class="space-y-4">
      <div class="space-y-2">
        <Label>Event Name</Label>
        <Input bind:value={eventName} placeholder="order.confirmed" class="font-mono" />
      </div>

      <div class="space-y-2">
        <Label>Data (JSON)</Label>
        <Textarea bind:value={dataJson} placeholder={'{"key": "value"}'} class="font-mono min-h-[120px]" />
      </div>

      <div>
        <button
          class="text-sm text-muted-foreground hover:text-foreground transition-colors underline"
          onclick={() => (showRecipients = !showRecipients)}
        >
          {showRecipients ? 'Hide recipient filter' : 'Target specific recipients'}
        </button>
      </div>

      {#if showRecipients}
        <div class="space-y-2">
          <Label>Recipient IDs (comma-separated)</Label>
          <Input bind:value={recipientIds} placeholder="user-1, user-2, user-3" class="font-mono" />
          <p class="text-xs text-muted-foreground">Leave empty to send to all recipients</p>
        </div>
      {/if}

      <Button class="w-full" onclick={handleSend} disabled={$sendBroadcast.isPending}>
        {$sendBroadcast.isPending ? 'Sending...' : 'Send Broadcast'}
      </Button>
    </Card.Content>
  </Card.Root>

  {#if error}
    <Card.Root class="max-w-2xl border-destructive/50">
      <Card.Content class="pt-6">
        <p class="text-sm text-destructive">{error}</p>
      </Card.Content>
    </Card.Root>
  {/if}

  {#if result}
    <Card.Root class="max-w-2xl">
      <Card.Header>
        <Card.Title>Broadcast Sent</Card.Title>
      </Card.Header>
      <Card.Content>
        <div class="space-y-3">
          <div class="flex justify-between text-sm">
            <span class="text-muted-foreground">Broadcast ID</span>
            <span class="font-mono">{result.broadcast_id}</span>
          </div>
          <div class="flex justify-between text-sm">
            <span class="text-muted-foreground">Recipients</span>
            <span class="font-mono">{result.recipient_count}</span>
          </div>
          <div class="flex justify-between text-sm">
            <span class="text-muted-foreground">Tasks Created</span>
            <span class="font-mono">{result.task_count}</span>
          </div>
        </div>
      </Card.Content>
    </Card.Root>
  {/if}
</div>
