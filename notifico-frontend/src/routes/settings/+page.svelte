<script lang="ts">
  import { createQuery } from '@tanstack/svelte-query';
  import { api } from '$lib/api/client';
  import type { Project, ChannelInfo } from '$lib/api/types';
  import { Badge } from '$lib/components/ui/badge';
  import * as Card from '$lib/components/ui/card';
  import * as Table from '$lib/components/ui/table';
  import { Separator } from '$lib/components/ui/separator';

  const projects = createQuery(() => ({
    queryKey: ['projects'],
    queryFn: () => api.get<Project[]>('/admin/api/v1/projects'),
  }));

  const channels = createQuery(() => ({
    queryKey: ['channels'],
    queryFn: () => api.get<ChannelInfo[]>('/admin/api/v1/channels'),
  }));
</script>

<div class="p-8 space-y-8">
  <div>
    <h1 class="text-2xl font-semibold tracking-tight">Settings</h1>
    <p class="text-sm text-muted-foreground mt-1">System configuration and available channels</p>
  </div>

  <!-- Projects -->
  <Card.Root>
    <Card.Header>
      <Card.Title>Projects</Card.Title>
      <Card.Description>Configured notification projects</Card.Description>
    </Card.Header>
    <Card.Content>
      {#if $projects.isLoading}
        <p class="text-sm text-muted-foreground py-4 text-center">Loading...</p>
      {:else if $projects.isError}
        <p class="text-sm text-destructive py-4 text-center">Failed to load projects</p>
      {:else if $projects.data}
        <Table.Root>
          <Table.Header>
            <Table.Row>
              <Table.Head>Name</Table.Head>
              <Table.Head>Default Locale</Table.Head>
              <Table.Head>Created</Table.Head>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {#each $projects.data as project}
              <Table.Row>
                <Table.Cell class="font-medium">{project.name}</Table.Cell>
                <Table.Cell>
                  <Badge variant="outline">{project.default_locale}</Badge>
                </Table.Cell>
                <Table.Cell class="text-sm text-muted-foreground">
                  {new Date(project.created_at).toLocaleDateString()}
                </Table.Cell>
              </Table.Row>
            {/each}
            {#if $projects.data.length === 0}
              <Table.Row>
                <Table.Cell colspan={3} class="text-center text-muted-foreground py-8">
                  No projects configured
                </Table.Cell>
              </Table.Row>
            {/if}
          </Table.Body>
        </Table.Root>
      {/if}
    </Card.Content>
  </Card.Root>

  <Separator />

  <!-- Available Channels -->
  <div>
    <h2 class="text-lg font-semibold tracking-tight mb-4">Available Channels</h2>
    {#if $channels.isLoading}
      <p class="text-sm text-muted-foreground py-4 text-center">Loading channels...</p>
    {:else if $channels.isError}
      <p class="text-sm text-destructive py-4 text-center">Failed to load channels</p>
    {:else if $channels.data}
      <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {#each $channels.data as channel}
          <Card.Root>
            <Card.Header>
              <Card.Title class="text-base">{channel.display_name}</Card.Title>
              <Card.Description class="font-mono">{channel.channel_id}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-4">
              {#if channel.content_schema.fields.length > 0}
                <div>
                  <h4 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2">Content Fields</h4>
                  <div class="space-y-1">
                    {#each channel.content_schema.fields as field}
                      <div class="flex items-center gap-2 text-sm">
                        <span class="font-mono">{field.name}</span>
                        <Badge variant="outline" class="text-xs">{field.field_type}</Badge>
                        {#if field.required}
                          <Badge variant="secondary" class="text-xs">required</Badge>
                        {/if}
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}

              {#if channel.credential_schema.fields.length > 0}
                <div>
                  <h4 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2">Credential Fields</h4>
                  <div class="space-y-1">
                    {#each channel.credential_schema.fields as field}
                      <div class="flex items-center gap-2 text-sm">
                        <span class="font-mono">{field.name}</span>
                        {#if field.required}
                          <Badge variant="secondary" class="text-xs">required</Badge>
                        {/if}
                        {#if field.secret}
                          <Badge variant="destructive" class="text-xs">secret</Badge>
                        {/if}
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </Card.Content>
          </Card.Root>
        {/each}
        {#if $channels.data.length === 0}
          <p class="text-sm text-muted-foreground col-span-full text-center py-8">
            No channels available
          </p>
        {/if}
      </div>
    {/if}
  </div>
</div>
