<script lang="ts">
  import { page } from '$app/stores';
  import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import type { Event, PipelineRule, MiddlewareEntry, EventStats, Template } from '$lib/api/types';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Badge } from '$lib/components/ui/badge';
  import { Textarea } from '$lib/components/ui/textarea';
  import * as Table from '$lib/components/ui/table';
  import * as Dialog from '$lib/components/ui/dialog';
  import * as Card from '$lib/components/ui/card';
  import * as Select from '$lib/components/ui/select';

  const queryClient = useQueryClient();
  let eventId = $derived($page.params.id);

  const event = createQuery(() => ({
    queryKey: ['events', eventId],
    queryFn: () => api.get<Event>(`/admin/api/v1/events/${eventId}`),
  }));

  const rules = createQuery(() => ({
    queryKey: ['events', eventId, 'rules'],
    queryFn: () => api.get<PipelineRule[]>(`/admin/api/v1/events/${eventId}/rules`),
  }));

  const templates = createQuery(() => ({
    queryKey: ['templates'],
    queryFn: () => api.get<Template[]>('/admin/api/v1/templates'),
  }));

  const stats = createQuery(() => ({
    queryKey: ['events', eventId, 'stats'],
    queryFn: () => api.get<EventStats>(`/admin/api/v1/events/${eventId}/stats`),
  }));

  // Add rule state
  let addRuleOpen = $state(false);
  let ruleChannel = $state('');
  let ruleTemplateId = $state('');
  let rulePriority = $state(0);

  const createRule = createMutation({
    mutationFn: (data: { channel: string; template_id: string; priority: number }) =>
      api.post(`/admin/api/v1/events/${eventId}/rules`, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['events', eventId, 'rules'] });
      addRuleOpen = false;
      ruleChannel = '';
      ruleTemplateId = '';
      rulePriority = 0;
    },
  });

  const deleteRule = createMutation({
    mutationFn: (ruleId: string) => api.delete(`/admin/api/v1/rules/${ruleId}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['events', eventId, 'rules'] });
    },
  });

  // Middleware state
  let expandedRule = $state<string | null>(null);
  let addMwOpen = $state(false);
  let mwRuleId = $state('');
  let mwName = $state('');
  let mwConfig = $state('{}');
  let mwPriority = $state(0);

  function middlewareQuery(ruleId: string) {
    return createQuery(() => ({
      queryKey: ['rules', ruleId, 'middleware'],
      queryFn: () => api.get<MiddlewareEntry[]>(`/admin/api/v1/rules/${ruleId}/middleware`),
      enabled: expandedRule === ruleId,
    }));
  }

  const createMiddleware = createMutation({
    mutationFn: (data: { rule_id: string; middleware_name: string; config: string; priority: number }) =>
      api.post(`/admin/api/v1/rules/${data.rule_id}/middleware`, {
        middleware_name: data.middleware_name,
        config: data.config,
        priority: data.priority,
      }),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['rules', variables.rule_id, 'middleware'] });
      addMwOpen = false;
      mwName = '';
      mwConfig = '{}';
      mwPriority = 0;
    },
  });

  const deleteMiddleware = createMutation({
    mutationFn: (data: { mwId: string; ruleId: string }) => api.delete(`/admin/api/v1/middleware/${data.mwId}`),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['rules', variables.ruleId, 'middleware'] });
    },
  });

  function toggleRule(ruleId: string) {
    expandedRule = expandedRule === ruleId ? null : ruleId;
  }

  function openAddMiddleware(ruleId: string) {
    mwRuleId = ruleId;
    addMwOpen = true;
  }

  // We need a reactive map of middleware queries for expanded rules
  let middlewareData = $state<Record<string, MiddlewareEntry[]>>({});

  $effect(() => {
    if (expandedRule && $rules.data) {
      api.get<MiddlewareEntry[]>(`/admin/api/v1/rules/${expandedRule}/middleware`).then((data) => {
        middlewareData[expandedRule!] = data;
      });
    }
  });
</script>

<div class="p-8 space-y-8">
  <!-- Header -->
  <div>
    <a href="/events" class="text-sm text-muted-foreground hover:text-foreground transition-colors">&larr; Back to Events</a>
    {#if $event.data}
      <h1 class="text-2xl font-semibold tracking-tight mt-2 font-mono">{$event.data.name}</h1>
      <Badge variant="secondary" class="mt-1">{$event.data.category}</Badge>
    {:else}
      <h1 class="text-2xl font-semibold tracking-tight mt-2">Loading...</h1>
    {/if}
  </div>

  <!-- Event Stats -->
  {#if $stats.data}
    <Card.Root>
      <Card.Header>
        <Card.Title>Delivery Statistics</Card.Title>
      </Card.Header>
      <Card.Content>
        <div class="flex gap-6">
          {#each $stats.data.stats as stat}
            <div class="text-center">
              <div class="text-2xl font-mono font-semibold">{stat.count}</div>
              <div class="text-sm text-muted-foreground">{stat.status}</div>
            </div>
          {/each}
          {#if $stats.data.stats.length === 0}
            <p class="text-sm text-muted-foreground">No delivery data yet</p>
          {/if}
        </div>
      </Card.Content>
    </Card.Root>
  {/if}

  <!-- Pipeline Rules -->
  <Card.Root>
    <Card.Header>
      <div class="flex items-center justify-between">
        <Card.Title>Pipeline Rules</Card.Title>
        <Dialog.Root bind:open={addRuleOpen}>
          <Dialog.Trigger>
            {#snippet children({ props })}
              <Button size="sm" {...props}>Add Rule</Button>
            {/snippet}
          </Dialog.Trigger>
          <Dialog.Content>
            <Dialog.Header>
              <Dialog.Title>Add Pipeline Rule</Dialog.Title>
              <Dialog.Description>Configure a delivery channel for this event</Dialog.Description>
            </Dialog.Header>
            <div class="space-y-4 py-4">
              <div class="space-y-2">
                <Label>Channel</Label>
                <Input bind:value={ruleChannel} placeholder="email" class="font-mono" />
              </div>
              <div class="space-y-2">
                <Label>Template</Label>
                {#if $templates.data && $templates.data.length > 0}
                  <Select.Root type="single" value={ruleTemplateId} onValueChange={(v) => { if (v) ruleTemplateId = v; }}>
                    <Select.Trigger class="w-full">
                      <span>{ruleTemplateId ? $templates.data.find((t) => t.id === ruleTemplateId)?.name ?? ruleTemplateId : 'Select template'}</span>
                    </Select.Trigger>
                    <Select.Content>
                      {#each $templates.data as template}
                        <Select.Item value={template.id}>{template.name} ({template.channel})</Select.Item>
                      {/each}
                    </Select.Content>
                  </Select.Root>
                {:else}
                  <Input bind:value={ruleTemplateId} placeholder="template ID" class="font-mono" />
                {/if}
              </div>
              <div class="space-y-2">
                <Label>Priority</Label>
                <Input type="number" bind:value={rulePriority} />
              </div>
            </div>
            <Dialog.Footer>
              <Button variant="outline" onclick={() => (addRuleOpen = false)}>Cancel</Button>
              <Button onclick={() => $createRule.mutate({ channel: ruleChannel, template_id: ruleTemplateId, priority: rulePriority })} disabled={$createRule.isPending}>
                {$createRule.isPending ? 'Adding...' : 'Add Rule'}
              </Button>
            </Dialog.Footer>
          </Dialog.Content>
        </Dialog.Root>
      </div>
    </Card.Header>
    <Card.Content>
      {#if $rules.isLoading}
        <p class="text-sm text-muted-foreground py-4 text-center">Loading rules...</p>
      {:else if $rules.data}
        <Table.Root>
          <Table.Header>
            <Table.Row>
              <Table.Head>Channel</Table.Head>
              <Table.Head>Template ID</Table.Head>
              <Table.Head>Priority</Table.Head>
              <Table.Head>Enabled</Table.Head>
              <Table.Head class="text-right">Actions</Table.Head>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {#each $rules.data as rule}
              <Table.Row class="cursor-pointer" onclick={() => toggleRule(rule.id)}>
                <Table.Cell>
                  <Badge variant="secondary">{rule.channel}</Badge>
                </Table.Cell>
                <Table.Cell class="font-mono text-sm">{rule.template_id}</Table.Cell>
                <Table.Cell class="font-mono">{rule.priority}</Table.Cell>
                <Table.Cell>
                  <Badge variant={rule.enabled ? 'default' : 'outline'}>
                    {rule.enabled ? 'on' : 'off'}
                  </Badge>
                </Table.Cell>
                <Table.Cell class="text-right">
                  <Button variant="ghost" size="sm" onclick={(e: MouseEvent) => { e.stopPropagation(); openAddMiddleware(rule.id); }}>
                    + Middleware
                  </Button>
                  <Button variant="ghost" size="sm" class="text-destructive" onclick={(e: MouseEvent) => { e.stopPropagation(); $deleteRule.mutate(rule.id); }}>
                    Delete
                  </Button>
                </Table.Cell>
              </Table.Row>
              <!-- Expanded middleware section -->
              {#if expandedRule === rule.id}
                <Table.Row>
                  <Table.Cell colspan={5} class="bg-muted/30 p-4">
                    <div class="space-y-2">
                      <h4 class="text-sm font-semibold">Middleware for {rule.channel}</h4>
                      {#if middlewareData[rule.id]}
                        {#each middlewareData[rule.id] as mw}
                          <div class="flex items-center justify-between rounded border border-border p-2 text-sm">
                            <div class="flex items-center gap-3">
                              <span class="font-mono">{mw.middleware_name}</span>
                              <Badge variant="outline">pri: {mw.priority}</Badge>
                            </div>
                            <Button variant="ghost" size="sm" class="text-destructive" onclick={() => $deleteMiddleware.mutate({ mwId: mw.id, ruleId: rule.id })}>
                              Remove
                            </Button>
                          </div>
                        {/each}
                        {#if middlewareData[rule.id].length === 0}
                          <p class="text-sm text-muted-foreground">No middleware configured</p>
                        {/if}
                      {:else}
                        <p class="text-sm text-muted-foreground">Loading middleware...</p>
                      {/if}
                    </div>
                  </Table.Cell>
                </Table.Row>
              {/if}
            {/each}
            {#if $rules.data.length === 0}
              <Table.Row>
                <Table.Cell colspan={5} class="text-center text-muted-foreground py-8">
                  No pipeline rules. Add one to configure delivery.
                </Table.Cell>
              </Table.Row>
            {/if}
          </Table.Body>
        </Table.Root>
      {/if}
    </Card.Content>
  </Card.Root>
</div>

<!-- Add Middleware Dialog -->
<Dialog.Root bind:open={addMwOpen}>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>Add Middleware</Dialog.Title>
      <Dialog.Description>Add processing middleware to this pipeline rule</Dialog.Description>
    </Dialog.Header>
    <div class="space-y-4 py-4">
      <div class="space-y-2">
        <Label>Middleware Name</Label>
        <Input bind:value={mwName} placeholder="rate_limiter" class="font-mono" />
      </div>
      <div class="space-y-2">
        <Label>Config (JSON)</Label>
        <Textarea bind:value={mwConfig} placeholder={'{}'} class="font-mono min-h-[100px]" />
      </div>
      <div class="space-y-2">
        <Label>Priority</Label>
        <Input type="number" bind:value={mwPriority} />
      </div>
    </div>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (addMwOpen = false)}>Cancel</Button>
      <Button onclick={() => $createMiddleware.mutate({ rule_id: mwRuleId, middleware_name: mwName, config: mwConfig, priority: mwPriority })} disabled={$createMiddleware.isPending}>
        {$createMiddleware.isPending ? 'Adding...' : 'Add Middleware'}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
