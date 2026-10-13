export function iso(d: Date): string { return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`; }
export function parseDate(s: string): Date { const [y,m,d]=s.split('-').map(Number); return new Date(y,m-1,d); }
export function addDays(d: Date, n: number): Date { const copy=new Date(d); copy.setDate(copy.getDate()+n); return copy; }
export function weekStart(d: Date): Date { return addDays(new Date(d.getFullYear(),d.getMonth(),d.getDate()), -((d.getDay()+6)%7)); }
const month=new Intl.DateTimeFormat('it-IT',{month:'long'});
export function weekLabel(start: Date): string { const end=addDays(start,6); const a=start.getDate(), b=end.getDate(); if(start.getMonth()===end.getMonth()) return `${a} — ${b} ${month.format(end)} ${end.getFullYear()}`; if(start.getFullYear()===end.getFullYear()) return `${a} ${month.format(start)} — ${b} ${month.format(end)} ${end.getFullYear()}`; return `${a} ${month.format(start)} ${start.getFullYear()} — ${b} ${month.format(end)} ${end.getFullYear()}`; }
