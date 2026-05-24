export type ConnectionState = "Connected" | "Degraded" | "Lost";
export type GroupKind = "Selector" | "UrlTest" | "Fallback" | "LoadBalance" | "Relay";

export interface NodeView {
  name: string;
  country: string;
  kind: string;
  latest_delay: number | null;
  is_group: boolean;
}

export interface GroupView {
  name: string;
  kind: GroupKind;
  now: string | null;
  now_country: string | null;
  now_delay: number | null;
  members: NodeView[];
  is_primary: boolean;
}

export interface AutoSwitch {
  group: string;
  from: string;
  to: string;
  from_country: string;
  to_country: string;
  at: number;
}

export interface TrafficStats {
  download_total: number;
  upload_total: number;
  download_speed: number;
  upload_speed: number;
  connection_count: number;
  memory: number | null;
}

export interface SubscriptionInfo {
  expiry: string | null;
  remaining: string | null;
  plan: string | null;
  homepage: string | null;
  reset: string | null;
}

export interface StateSnapshot {
  fetched_at: number;
  connection: ConnectionState;
  groups: GroupView[];
  leaf_count: number;
  recent_auto_switches: AutoSwitch[];
  traffic: TrafficStats;
  subscription: SubscriptionInfo;
}

export interface Settings {
  clash_url: string;
  secret: string | null;
  poll_interval_ms: number;
  primary_group: string | null;
  favorites: string[];
  country_overrides: Record<string, string>;
  notify_auto_switch: boolean;
  notify_connection: boolean;
  start_on_login: boolean;
}

export interface VersionInfo {
  version: string;
  premium: boolean;
  meta: boolean;
}
