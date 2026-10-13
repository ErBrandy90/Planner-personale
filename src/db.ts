import { invoke as tauriInvoke } from '@tauri-apps/api/core';
const pending = new Set<Promise<unknown>>();
function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const p = tauriInvoke<T>(command, args);
  if (command !== 'load_week' && command !== 'inspect_import') {
    pending.add(p);
    void p.then(
      () => pending.delete(p),
      () => pending.delete(p),
    );
  }
  return p;
}
export async function flushWrites(): Promise<void> {
  while (pending.size) await Promise.all([...pending]);
}
import { parseEntry, type EntryKind } from './parser';
export type Entry = {
  id: number;
  week_start: string;
  day_date: string;
  content: string;
  position: number;
  detected_time: string | null;
  scheduled_time: string | null;
  kind: EntryKind;
  completed: boolean;
};
export type WeekData = { entries: Entry[]; notes: string };
export async function loadWeek(weekStart: string): Promise<WeekData> {
  return invoke('load_week', { weekStart });
}
export async function addEntry(weekStart: string, dayDate: string, content: string, position: number): Promise<Entry> {
  return invoke('add_entry', { weekStart, dayDate, content, position, ...parseEntry(content) });
}
export async function updateEntry(id: number, content: string): Promise<Entry> {
  return invoke('update_entry', { id, content, ...parseEntry(content) });
}
export async function setCompleted(id: number, completed: boolean): Promise<void> {
  return invoke('set_completed', { id, completed });
}
export async function deleteEntry(id: number): Promise<void> {
  return invoke('delete_entry', { id });
}
export async function duplicateEntry(id: number): Promise<Entry> {
  return invoke('duplicate_entry', { id });
}
export async function moveEntry(id: number, dayDate: string): Promise<Entry> {
  return invoke('move_entry', { id, dayDate });
}
export async function setEntryTime(id: number, time: string | null): Promise<Entry> {
  return invoke('set_entry_time', { id, time });
}
// base = note lette dal database prima di modificarle; restituisce il testo salvato, con le eventuali righe aggiunte da fuori
export async function saveNotes(weekStart: string, content: string, base: string): Promise<string> {
  return invoke('save_notes', { weekStart, content, base });
}
export async function exportData(path: string): Promise<void> {
  return invoke('export_data', { path });
}

export type ImportPreview = { content: string; entries: number; notes: number };
export type ImportReport = { added: number; skipped: number; notes_updated: number; backup_path: string };
export async function inspectImport(path: string): Promise<ImportPreview> {
  return invoke('inspect_import', { path });
}
export async function importData(content: string): Promise<ImportReport> {
  return invoke('import_data', { content });
}
export async function backupData(): Promise<string> {
  return invoke('backup_data');
}
