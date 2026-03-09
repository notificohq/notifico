<script lang="ts">
  import { createQuery } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import type { DeliveryLogResponse } from '$lib/api/types';
  import * as Card from '$lib/components/ui/card';
  import * as Table from '$lib/components/ui/table';
  import { Badge } from '$lib/components/ui/badge';

  const recentLog = createQuery(() => ({
    queryKey: ['delivery-log', 'recent'],
    queryFn: () => api.get<DeliveryLogResponse>('/admin/api/v1/delivery-log?limit=20'),
  }));

  const deliveredCount = createQuery(() => ({
    queryKey: ['delivery-log', 'count', 'delivered'],
    queryFn: () => api.get<DeliveryLogResponse>('/admin/api/v1/delivery-log?status=delivered&limit=0'),
  }));

  const failedCount = createQuery(() => ({
    queryKey: ['delivery-log', 'count', 'failed'],
    queryFn: () => api.get<DeliveryLogResponse>('/admin/api/v1/delivery-log?status=failed&limit=0'),
  }));

  const queuedCount = createQuery(() => ({
    queryKey: ['delivery-log', 'count', 'queued'],
    queryFn: () => api.get<DeliveryLogResponse>('/admin/api/v1/delivery-log?status=queued&limit=0'),
  }));

  const deadLetterCount = createQuery(() => ({
    queryKey: ['delivery-log', 'count', 'dead_letter'],
    queryFn: () => api.get<DeliveryLogResponse>('/admin/api/v1/delivery-log?status=dead_letter&limit=0'),
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
</script>

<div class="p-8 space-y-8">
  <div>
    <h1 class="text-2xl font-semibold tracking-tight">Dashboard</h1>
    <p class="text-sm text-muted-foreground mt-1">System overview</p>
  </div>

  <!-- Stats Cards -->
  <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
    <Card.Root>
      <Card.Header>
        <Card.Description>Delivered</Card.Description>
        <Card.Title class="text-3xl font-mono text-green-500">
          {$deliveredCount.data?.total ?? '---'}
        </Card.Title>
      </Card.Header>
    </Card.Root>

    <Card.Root>
      <Card.Header>
        <Card.Description>Failed</Card.Description>
        <Card.Title class="text-3xl font-mono text-red-500">
          {$failedCount.data?.total ?? '---'}
        </Card.Title>
      </Card.Header>
    </Card.Root>

    <Card.Root>
      <Card.Header>
        <Card.Description>Queued</Card.Description>
        <Card.Title class="text-3xl font-mono text-amber-500">
          {$queuedCount.data?.total ?? '---'}
        </Card.Title>
      </Card.Header>
    </Card.Root>

    <Card.Root>
      <Card.Header>
        <Card.Description>Dead Letter</Card.Description>
        <Card.Title class="text-3xl font-mono text-muted-foreground">
          {$deadLetterCount.data?.total ?? '---'}
        </Card.Title>
      </Card.Header>
    </Card.Root>
  </div>

  <!-- Recent Deliveries -->
  <Card.Root>
    <Card.Header>
      <Card.Title>Recent Deliveries</Card.Title>
      <Card.Description>Last 20 delivery log entries</Card.Description>
    </Card.Header>
    <Card.Content>
      {#if $recentLog.isLoading}
        <p class="text-sm text-muted-foreground py-8 text-center">Loading...</p>
      {:else if $recentLog.isError}
        <p class="text-sm text-destructive py-8 text-center">Failed to load delivery log</p>
      {:else if $recentLog.data}
        <Table.Root>
          <Table.Header>
            <Table.Row>
              <Table.Head>Event</Table.Head>
              <Table.Head>Channel</Table.Head>
              <Table.Head>Status</Table.Head>
              <Table.Head>Created</Table.Head>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {#each $recentLog.data.items as entry}
              <Table.Row>
                <Table.Cell class="font-mono text-sm">{entry.event_name}</Table.Cell>
                <Table.Cell>
                  <Badge variant="secondary">{entry.channel}</Badge>
                </Table.Cell>
                <Table.Cell>
                  <Badge variant={statusColor(entry.status)}>{entry.status}</Badge>
                </Table.Cell>
                <Table.Cell class="text-muted-foreground text-sm">{formatDate(entry.created_at)}</Table.Cell>
              </Table.Row>
            {/each}
            {#if $recentLog.data.items.length === 0}
              <Table.Row>
                <Table.Cell colspan={4} class="text-center text-muted-foreground py-8">
                  No delivery log entries yet
                </Table.Cell>
              </Table.Row>
            {/if}
          </Table.Body>
        </Table.Root>
      {/if}
    </Card.Content>
  </Card.Root>
</div>
