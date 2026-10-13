export type EntryKind = 'text' | 'task';
export function parseEntry(content: string): { kind: EntryKind; detected_time: string | null } {
  const first = content.trimStart().split('\n')[0];
  const kind = /^\[\s?\]\s*/.test(first) ? 'task' : 'text';
  const time = first.match(/^(?:\[\s?\]\s*)?([01]?\d|2[0-3]):([0-5]\d)(?=\s|$)/);
  return { kind, detected_time: time ? `${time[1].padStart(2, '0')}:${time[2]}` : null };
}
export function displayText(content: string, kind: EntryKind): string {
  return kind === 'task' ? content.replace(/^\[\s?\]\s*/, '') : content;
}
