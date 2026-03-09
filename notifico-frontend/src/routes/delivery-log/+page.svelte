<script lang="ts">
  import { createQuery } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import type { DeliveryLogResponse } from '$lib/api/types';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Badge } from '$lib/components/ui/badge';
  import * as Table from '$lib/components/ui/table';
  import * as Select from '$lib/components/ui/select';

  let statusFilter = $state('all');
  let eventFilter = $state('');
  let currentPage = $state(0);
  let autoRefresh = $state(false);
  const pageSize = 20;

  let queryParams = $derived(() => {
    const params = new URLSearchParams();
    params.set('limit', String(pageSize));
    params.set('offset', String(currentPage * pageSize));
    if (statusFilter !== 'all') params.set('status', statusFilter);
    if (eventFilter.trim()) params.set('event_name', eventFilter.trim());
    return params.toString();
  });

  const deliveryLog = createQuery(() => ({
    queryKey: ['delivery-log', statusFilter, eventFilter, currentPage],
    queryFn: () => api.get<DeliveryLogResponse>(`/admin/api/v1/delivery-log?${queryParams()}`),
    refetchInterval: autoRefresh ? 5000 : false,
  }));

  function statusColor(status: string): 'default' | 'secondary' | 'destructive' | 'outline' {
    switch (status) {
      case 'delivered': return 'default';
      case 'failed': return 'destructive';
      case 'queued': return 'secondary';
      case 'dead_letter': return 'outline';
      default: return 'secondary';
    }
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleString();
  }

  function truncate(str: string, len: number): string {
    if (str.length <= len) return str;
    return str.substring(0, len) + '...';
  }

  let totalPages = $derived($deliveryLog.data ? Math.ceil($deliveryLog.data.total / pageSize) : 0);
</script>

<div class="p-8 space-y-6">
  <div>
    <h1 class="text-2xl font-semibold tracking-tight">Delivery Log</h1>
    <p class="text-sm text-muted-foreground mt-1">View notification delivery history</p>
  </div>

  <!-- Filters -->
  <div class="flex flex-wrap items-center gap-4">
    <div class="flex items-center gap-2">
      <span class="text-sm text-muted-foreground">Status:</span>
      <Select.Root type="single" value={statusFilter} onValueChange={(v) => { if (v) { statusFilter = v; currentPage = 0; } }}>
        <Select.Trigger class="w-[140px]">
          <span>{statusFilter}</span>
        </Select.Trigger>
        <Select.Content>
          <Select.Item value="all">all</Select.Item>
          <Select.Item value="delivered">delivered</Select.Item>
          <Select.Item value="failed">failed</Select.Item>
          <Select.Item value="queued">queued</Select.Item>
          <Select.Item value="dead_letter">dead_letter</Select.Item>
        </Select.Content>
      </Select.Root>
    </div>

    <div class="flex items-center gap-2">
      <span class="text-sm text-muted-foreground">Event:</span>
      <Input
        bind:value={eventFilter}
        placeholder="Filter by event name"
        class="w-[200px] font-mono"
        oninput={() => (currentPage = 0)}
      />
    </div>

    <label class="flex items-center gap-2 text-sm cursor-pointer ml-auto">
      <input type="checkbox" bind:checked={autoRefresh} class="rounded border-border" />
      <span class="text-muted-foreground">Auto-refresh (5s)</span>
    </label>
  </div>

  <!-- Table -->
  {#if $deliveryLog.isLoading}
    <p class="text-sm text-muted-foreground py-8 text-center">Loading...</p>
  {:else if $deliveryLog.isError}
    <p class="text-sm text-destructive py-8 text-center">Failed to load delivery log</p>
  {:else if $deliveryLog.data}
    <div class="text-sm text-muted-foreground">
      {$deliveryLog.data.total} total entries
    </div>

    <Table.Root>
      <Table.Header>
        <Table.Row>
          <Table.Head>Event</Table.Head>
          <Table.Head>Channel</Table.Head>
          <Table.Head>Status</Table.Head>
          <Table.Head>Recipient</Table.Head>
          <Table.Head>Error</Table.Head>
          <Table.Head>Created</Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#each $deliveryLog.data.items as entry}
          <Table.Row>
            <Table.Cell class="font-mono text-sm">{entry.event_name}</Table.Cell>
            <Table.Cell>
              <Badge variant="secondary">{entry.channel}</Badge>
            </Table.Cell>
            <Table.Cell>
              <Badge variant={statusColor(entry.status)}>{entry.status}</Badge>
            </Table.Cell>
            <Table.Cell class="font-mono text-sm text-muted-foreground">{truncate(entry.recipient_id, 12)}</Table.Cell>
            <Table.Cell class="text-sm text-destructive max-w-[200px] truncate">
              {entry.error_message ?? ''}
            </Table.Cell>
            <Table.Cell class="text-sm text-muted-foreground">{formatDate(entry.created_at)}</Table.Cell>
          </Table.Row>
        {/each}
        {#if $deliveryLog.data.items.length === 0}
          <Table.Row>
            <Table.Cell colspan={6} class="text-center text-muted-foreground py-8">
              No delivery log entries found
            </Table.Cell>
          </Table.Row>
        {/if}
      </Table.Body>
    </Table.Root>

    <!-- Pagination -->
    {#if totalPages > 1}
      <div class="flex items-center justify-center gap-4 pt-4">
        <Button variant="outline" size="sm" disabled={currentPage === 0} onclick={() => (currentPage -= 1)}>
          Previous
        </Button>
        <span class="text-sm text-muted-foreground">
          Page {currentPage + 1} of {totalPages}
        </span>
        <Button variant="outline" size="sm" disabled={currentPage >= totalPages - 1} onclick={() => (currentPage += 1)}>
          Next
        </Button>
      </div>
    {/if}
  {/if}
</div>
