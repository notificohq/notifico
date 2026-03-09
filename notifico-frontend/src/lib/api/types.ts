export interface Project {
  id: string;
  name: string;
  default_locale: string;
  created_at: string;
  updated_at: string;
}

export interface Event {
  id: string;
  project_id: string;
  name: string;
  category: string;
  description?: string;
}

export interface PipelineRule {
  id: string;
  event_id: string;
  channel: string;
  template_id: string;
  enabled: boolean;
  priority: number;
}

export interface Template {
  id: string;
  project_id: string;
  name: string;
  channel: string;
}

export interface Recipient {
  id: string;
  project_id: string;
  external_id: string;
  locale: string;
  timezone?: string;
  metadata?: Record<string, unknown>;
}

export interface Contact {
  id: string;
  recipient_id: string;
  channel: string;
  value: string;
}

export interface Credential {
  id: string;
  name: string;
  channel: string;
  enabled: boolean;
}

export interface DeliveryLogEntry {
  id: string;
  project_id: string;
  event_name: string;
  recipient_id: string;
  channel: string;
  status: string;
  error_message?: string;
  attempts: number;
  created_at: string;
}

export interface DeliveryLogResponse {
  items: DeliveryLogEntry[];
  total: number;
}

export interface ApiKey {
  id: string;
  name: string;
  scope: string;
  prefix: string;
  created_at: string;
  raw_key?: string;
}

export interface ChannelInfo {
  channel_id: string;
  display_name: string;
  content_schema: { fields: ContentField[] };
  credential_schema: { fields: CredentialField[] };
}

export interface ContentField {
  name: string;
  field_type: string;
  required: boolean;
  description: string;
}

export interface CredentialField {
  name: string;
  required: boolean;
  secret: boolean;
  description: string;
}

export interface MiddlewareEntry {
  id: string;
  rule_id: string;
  middleware_name: string;
  config: string;
  priority: number;
  enabled: boolean;
}

export interface EventStats {
  event_id: string;
  stats: { status: string; count: number }[];
}
