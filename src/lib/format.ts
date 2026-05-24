/** Format bytes as a short human-readable string (e.g. "1.2 MB", "445 GB"). */
export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return "—";
  if (n < 1024) return `${n} B`;
  const kb = n / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  const gb = mb / 1024;
  return `${gb.toFixed(2)} GB`;
}

/** Format a byte-rate as "X/s". */
export function formatSpeed(n: number): string {
  if (!Number.isFinite(n) || n < 0) return "—";
  return `${formatBytes(n)}/s`;
}
