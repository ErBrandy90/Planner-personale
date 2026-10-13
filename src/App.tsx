import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { addDays, iso, parseDate, weekLabel, weekStart } from './date';
import { getVersion } from '@tauri-apps/api/app';
import { open as openFile, save } from '@tauri-apps/plugin-dialog';
import * as db from './db';
import { UpdateDialog } from './UpdateDialog';
import { displayText } from './parser';

const names=['Lunedì','Martedì','Mercoledì','Giovedì','Venerdì','Sabato','Domenica'];
const rowHeight=33;
const currentWeek=()=>iso(weekStart(new Date()));
type ThemePreference='light'|'dark'|'system';
type DialogName='about'|'changelog'|null;

const changes=[
  {version:'0.4.0',items:['Importazione JSON con anteprima, unione dei dati e backup automatico','Controllo e installazione degli aggiornamenti dal menu Aiuto','Pacchetti di aggiornamento firmati e installer per ogni versione']},
  {version:'0.3.1',items:['Migliorato il contrasto dei colori nelle intestazioni del tema scuro']},
  {version:'0.3.0',items:['Barra dei menu con Informazioni e Changelog','Temi chiaro, scuro e sistema','Il connettore MCP può aggiungere impegni alle note settimanali']},
  {version:'0.2.0',items:['Connettore MCP per Claude Desktop e altri assistenti','Sincronizzazione delle modifiche esterne mentre il planner è aperto']},
  {version:'0.1.0',items:['Planner settimanale offline con SQLite','Scrittura diretta, checkbox, orari, note ed esportazione JSON']}
];

function MenuBar({theme,onTheme,onExport,onImport,onUpdate,exportReady,onDialog}:{theme:ThemePreference;onTheme:(theme:ThemePreference)=>void;onExport:()=>void;onImport:()=>void;onUpdate:()=>void;exportReady:boolean;onDialog:(dialog:DialogName)=>void}){
  const [open,setOpen]=useState<string|null>(null); const ref=useRef<HTMLDivElement>(null);
  useEffect(()=>{const close=(event:PointerEvent)=>{if(!ref.current?.contains(event.target as Node))setOpen(null);};document.addEventListener('pointerdown',close);return()=>document.removeEventListener('pointerdown',close);},[]);
  const chooseTheme=(value:ThemePreference)=>{onTheme(value);setOpen(null);};
  return <div className="menu-bar" ref={ref}>
    <div className="menu-group"><button className="menu-trigger" onClick={()=>setOpen(open==='file'?null:'file')}>File</button>{open==='file'&&<div className="menu-dropdown"><button disabled={!exportReady} onClick={()=>{setOpen(null);onExport();}}>Esporta dati…</button><button disabled={!exportReady} onClick={()=>{setOpen(null);onImport();}}>Importa dati…</button></div>}</div>
    <div className="menu-group"><button className="menu-trigger" onClick={()=>setOpen(open==='view'?null:'view')}>Visualizza</button>{open==='view'&&<div className="menu-dropdown"><div className="menu-caption">Tema</div>{([['light','Chiaro'],['dark','Scuro'],['system','Sistema']] as const).map(([value,label])=><button key={value} onClick={()=>chooseTheme(value)}><span className="menu-check">{theme===value?'✓':''}</span>{label}</button>)}</div>}</div>
    <div className="menu-group"><button className="menu-trigger" onClick={()=>setOpen(open==='help'?null:'help')}>Aiuto</button>{open==='help'&&<div className="menu-dropdown"><button onClick={()=>{setOpen(null);onDialog('about');}}>Informazioni sul software</button><button onClick={()=>{setOpen(null);onDialog('changelog');}}>Changelog</button><button disabled={!exportReady} onClick={()=>{setOpen(null);onUpdate();}}>Controlla aggiornamenti…</button></div>}</div>
  </div>;
}

function InfoDialog({name,version,onClose}:{name:Exclude<DialogName,null>;version:string;onClose:()=>void}){
  useEffect(()=>{const close=(event:KeyboardEvent)=>{if(event.key==='Escape')onClose();};window.addEventListener('keydown',close);return()=>window.removeEventListener('keydown',close);},[onClose]);
  return <div className="modal-backdrop" onMouseDown={event=>{if(event.target===event.currentTarget)onClose();}}><section className="modal" role="dialog" aria-modal="true" aria-labelledby="dialog-title">
    <button className="modal-close" onClick={onClose} aria-label="Chiudi">×</button>
    {name==='about'?<><h2 id="dialog-title">Planner Settimanale</h2><p className="version">Versione {version}</p><p>Un planner desktop locale, rapido e senza account. I dati sono salvati sul computer in SQLite.</p><dl><div><dt>Interfaccia</dt><dd>React e TypeScript</dd></div><div><dt>Applicazione desktop</dt><dd>Tauri e Rust</dd></div><div><dt>Integrazioni AI</dt><dd>Model Context Protocol (MCP)</dd></div></dl></>:<><h2 id="dialog-title">Changelog</h2><div className="changelog">{changes.map(change=><section key={change.version}><h3>Versione {change.version}</h3><ul>{change.items.map(item=><li key={item}>{item}</li>)}</ul></section>)}</div></>}
  </section></div>;
}

function WeekNavigation({week,onChange}:{week:string;onChange:(week:string)=>void}){
  const base=parseDate(week);
  return <header className="topbar"><div className="brand">SETTIMANA</div><nav aria-label="Navigazione settimane"><button aria-label="Settimana precedente" onClick={()=>onChange(iso(addDays(base,-7)))}>‹</button><strong>{weekLabel(base)}</strong><button aria-label="Settimana successiva" onClick={()=>onChange(iso(addDays(base,7)))}>›</button></nav><div className="top-actions"><button className="today" onClick={()=>onChange(currentWeek())}>Oggi</button></div></header>;
}

function EntryEditor({initial,onSave,onCancel}:{initial:string;onSave:(value:string)=>Promise<void>;onCancel:()=>void}){
  const [value,setValue]=useState(initial); const settled=useRef(false); const editor=useRef<HTMLTextAreaElement>(null);
  useLayoutEffect(()=>{if(editor.current){editor.current.style.height='33px';editor.current.style.height=`${Math.max(33,editor.current.scrollHeight)}px`;}},[value]);
  const submit=async()=>{if(settled.current)return;settled.current=true;await onSave(value.trim());};
  return <textarea ref={editor} className="entry-editor" autoFocus rows={1} value={value} onChange={e=>setValue(e.target.value)} onKeyDown={e=>{if(e.key==='Enter'&&!e.shiftKey){e.preventDefault();void submit();}if(e.key==='Escape'){e.preventDefault();settled.current=true;onCancel();}}} onBlur={()=>void submit()} aria-label="Testo della voce"/>;
}

function PlannerEntry({entry,onUpdate,onToggle,onDelete,onDuplicate,onMove,onSetTime}:{entry:db.Entry;onUpdate:(entry:db.Entry,value:string)=>Promise<void>;onToggle:(entry:db.Entry)=>Promise<void>;onDelete:(entry:db.Entry)=>Promise<void>;onDuplicate:(entry:db.Entry)=>Promise<void>;onMove:(entry:db.Entry,day:string)=>Promise<void>;onSetTime:(entry:db.Entry,time:string|null)=>Promise<void>}){
  const [editing,setEditing]=useState(false),[menu,setMenu]=useState(false),[time,setTime]=useState(entry.scheduled_time??entry.detected_time??'20:00');
  const menuRef=useRef<HTMLDivElement>(null);
  useEffect(()=>{if(!menu)return;const close=(e:PointerEvent)=>{if(!menuRef.current?.contains(e.target as Node))setMenu(false);};document.addEventListener('pointerdown',close);return()=>document.removeEventListener('pointerdown',close);},[menu]);
  if(editing)return <EntryEditor initial={entry.content} onSave={async value=>{if(value)await onUpdate(entry,value);else await onDelete(entry);setEditing(false);}} onCancel={()=>setEditing(false)}/>;
  return <div className={`entry ${entry.completed?'done':''}`} onContextMenu={e=>{e.preventDefault();setMenu(true);}}>
    {entry.kind==='task'&&<input type="checkbox" checked={entry.completed} onChange={()=>void onToggle(entry)} aria-label="Attività completata"/>}
    {entry.scheduled_time&&<span className="entry-time">{entry.scheduled_time}</span>}
    <button className="entry-text" onClick={()=>setEditing(true)}>{displayText(entry.content,entry.kind)}</button>
    {menu&&<div className="context" role="menu" ref={menuRef}>
      <button onClick={()=>{setMenu(false);setEditing(true);}}>Modifica</button>
      {entry.kind==='task'&&<button onClick={()=>{setMenu(false);void onToggle(entry);}}>{entry.completed?'Segna da fare':'Segna completata'}</button>}
      <div className="move-label">Sposta a…</div>
      <div className="move-days">{names.map((name,i)=><button key={name} title={name} onClick={()=>{setMenu(false);void onMove(entry,iso(addDays(parseDate(entry.week_start),i)));}}>{name.slice(0,3)}</button>)}</div>
      <div className="move-label">Orario</div>
      <div className="time-control"><input type="time" value={time} onChange={e=>setTime(e.target.value)} aria-label="Orario dell'impegno"/><button onClick={()=>{if(time){setMenu(false);void onSetTime(entry,time);}}}>Imposta</button></div>
      {entry.scheduled_time&&<button onClick={()=>{setMenu(false);void onSetTime(entry,null);}}>Rimuovi orario</button>}
      <button onClick={()=>{setMenu(false);void onDuplicate(entry);}}>Duplica</button>
      <button onClick={()=>{setMenu(false);void onDelete(entry);}}>Elimina</button>
    </div>}
  </div>;
}

function DayColumn({date,index,entries,actions}:{date:string;index:number;entries:db.Entry[];actions:Actions}){
  const [addingAt,setAddingAt]=useState<number|null>(null); const isToday=date===iso(new Date());
  const items=[...entries.map(entry=>({position:entry.position,entry})),...(addingAt===null?[]:[{position:addingAt,entry:null}])].sort((a,b)=>a.position-b.position || (a.entry===null?-1:1));
  let cursor=0;
  return <section className={`day day-${index} ${isToday?'is-today':''}`} aria-label={`${names[index]} ${parseDate(date).getDate()}`}>
    <div className="day-heading"><span className="day-name">{names[index]}</span><span className="day-number">{parseDate(date).getDate()}</span></div>
    <div className="day-body" onClick={e=>{if(e.target===e.currentTarget){const rect=e.currentTarget.getBoundingClientRect();setAddingAt(Math.max(0,Math.floor((e.clientY-rect.top+e.currentTarget.scrollTop)/rowHeight)));}}}>
      {items.map(item=>{const gap=Math.max(0,item.position-cursor)*rowHeight;cursor=Math.max(cursor,item.position)+Math.max(1,item.entry?.content.split('\n').length??1);return <div className="entry-slot" style={{marginTop:gap}} key={item.entry?.id??'new'}>{item.entry?<PlannerEntry entry={item.entry} {...actions}/>:<EntryEditor initial="" onSave={async value=>{if(value)await actions.onAdd(date,value,item.position);setAddingAt(null);}} onCancel={()=>setAddingAt(null)}/>}</div>;})}
      {addingAt===null&&<button className="add-row" aria-label={`Scrivi in ${names[index]}`} onClick={()=>setAddingAt(cursor)}>+ Scrivi qui</button>}
    </div>
  </section>;
}
type Actions={onAdd:(day:string,value:string,position:number)=>Promise<void>;onUpdate:(entry:db.Entry,value:string)=>Promise<void>;onToggle:(entry:db.Entry)=>Promise<void>;onDelete:(entry:db.Entry)=>Promise<void>;onDuplicate:(entry:db.Entry)=>Promise<void>;onMove:(entry:db.Entry,day:string)=>Promise<void>;onSetTime:(entry:db.Entry,time:string|null)=>Promise<void>};

type NotesHandle={flush:()=>Promise<void>};
// Le note si salvano insieme alla "base" (ultimo testo letto dal database): se nel frattempo il connettore MCP
// ha aggiunto righe, il backend le unisce in fondo invece di cancellarle e restituisce il testo finale.
function WeeklyNotes({week,value,ready,base,onChange,onSaved,handle,onError}:{week:string;value:string;ready:boolean;base:(week:string)=>string;onChange:(v:string)=>void;onSaved:(week:string,sent:string,saved:string)=>void;handle:React.MutableRefObject<NotesHandle|null>;onError:(error:string)=>void}){
  const timer=useRef<ReturnType<typeof setTimeout>|null>(null);const pending=useRef<string|null>(null);
  const area=useRef<HTMLTextAreaElement>(null);const typed=useRef(value);const selection=useRef<[number,number]>([0,0]);
  // I salvataggi partono uno alla volta: ognuno usa la base aggiornata dal precedente.
  const queue=useRef<Promise<void>>(Promise.resolve());
  const flush=useCallback(async()=>{if(timer.current)clearTimeout(timer.current);const run=queue.current.then(async()=>{if(pending.current===null)return;const v=pending.current;const saved=await db.saveNotes(week,v,base(week));if(pending.current===v)pending.current=null;onSaved(week,v,saved);});queue.current=run.catch(()=>{});await run;await db.flushWrites();},[week,base,onSaved]);
  useLayoutEffect(()=>{handle.current={flush};return()=>{handle.current=null;};},[handle,flush]);
  useEffect(()=>()=>{void flush().catch(e=>onError(`Salvataggio note non riuscito: ${String(e)}`));},[flush,onError]);
  // Testo aggiornato da fuori mentre si scrive: il cursore resta dov'era invece di saltare alla fine.
  useLayoutEffect(()=>{const el=area.current;if(!el||value===typed.current)return;typed.current=value;if(document.activeElement===el){const [start,end]=selection.current;el.setSelectionRange(Math.min(start,value.length),Math.min(end,value.length));}},[value]);
  const remember=()=>{const el=area.current;if(el)selection.current=[el.selectionStart,el.selectionEnd];};
  const saveLater=()=>{void flush().catch(e=>onError(`Salvataggio note non riuscito: ${String(e)}`));};
  return <section className="notes"><label htmlFor="weekly-notes">NOTE</label><textarea ref={area} id="weekly-notes" value={value} readOnly={!ready} placeholder="Uno spazio per tutto il resto della settimana…" onSelect={remember} onChange={e=>{const v=e.target.value;typed.current=v;remember();onChange(v);pending.current=v;if(timer.current)clearTimeout(timer.current);timer.current=setTimeout(saveLater,250);}} onBlur={saveLater}/></section>;
}

export default function App(){
  const [week,setWeek]=useState(currentWeek); const [data,setData]=useState<db.WeekData>({entries:[],notes:''}); const [error,setError]=useState(''); const [status,setStatus]=useState(''); const [loadedWeek,setLoadedWeek]=useState<string|null>(null); const [dialog,setDialog]=useState<DialogName>(null); const [theme,setTheme]=useState<ThemePreference>(()=>(localStorage.getItem('planner-theme') as ThemePreference)||'system'); const loadId=useRef(0); const notesHandle=useRef<NotesHandle|null>(null); const [importPreview,setImportPreview]=useState<db.ImportPreview|null>(null); const [importBusy,setImportBusy]=useState(false); const [updatesOpen,setUpdatesOpen]=useState(false); const operation=useRef(false);
  // La versione si legge dall'app (tauri.conf.json → package.json), non è scritta nel codice.
  const [appVersion,setAppVersion]=useState('…'); useEffect(()=>{getVersion().then(setAppVersion).catch(()=>setAppVersion('sconosciuta'));},[]);
  useEffect(()=>{localStorage.setItem('planner-theme',theme);const media=window.matchMedia('(prefers-color-scheme: dark)');const apply=()=>{document.documentElement.dataset.theme=theme==='system'?(media.matches?'dark':'light'):theme;};apply();media.addEventListener('change',apply);return()=>media.removeEventListener('change',apply);},[theme]);
  // Note: notesBase = ultimo testo letto o salvato nel database per quella settimana; notesValue = testo nel campo.
  // Se sono diversi ci sono modifiche non ancora salvate, e il testo letto dal database non deve sostituirle.
  const notesBase=useRef({week:'',content:''}); const notesValue=useRef(''); const weekRef=useRef(week); weekRef.current=week;
  const setNotes=useCallback((notes:string)=>{notesValue.current=notes;setData(d=>({...d,notes}));},[]);
  const applyWeek=useCallback((forWeek:string,v:db.WeekData)=>{if(forWeek!==weekRef.current)return;const unsaved=notesBase.current.week!==forWeek||notesValue.current!==notesBase.current.content;if(!unsaved){notesBase.current={week:forWeek,content:v.notes};notesValue.current=v.notes;}setData(d=>({entries:v.entries,notes:unsaved?d.notes:v.notes}));},[]);
  const notesBaseFor=useCallback((w:string)=>notesBase.current.week===w?notesBase.current.content:'',[]);
  const onNotesSaved=useCallback((w:string,sent:string,saved:string)=>{if(notesBase.current.week!==w||weekRef.current!==w)return;if(saved!==sent&&notesValue.current===sent){setNotes(saved);notesBase.current={week:w,content:saved};}else notesBase.current={week:w,content:sent};},[setNotes]);
  useEffect(()=>{const id=++loadId.current;setLoadedWeek(null);db.loadWeek(week).then(v=>{if(id===loadId.current){notesBase.current={week,content:v.notes};notesValue.current=v.notes;setData(v);setLoadedWeek(week);setError('');}}).catch(e=>setError(String(e)));},[week]);
  useEffect(()=>{
    let active=true;
    const syncExternal=()=>{if(operation.current||updatesOpen)return;db.loadWeek(week).then(v=>{if(active)applyWeek(week,v);}).catch(()=>{});};
    const timer=setInterval(syncExternal,3000);
    window.addEventListener('focus',syncExternal);
    return()=>{active=false;clearInterval(timer);window.removeEventListener('focus',syncExternal);};
  },[week,updatesOpen,applyWeek]);
  useEffect(()=>{const onKey=(e:KeyboardEvent)=>{if(operation.current||updatesOpen)return;if(e.ctrlKey&&e.key.toLowerCase()==='t'){e.preventDefault();setWeek(currentWeek());}if(e.altKey&&e.key==='ArrowLeft'){e.preventDefault();setWeek(w=>iso(addDays(parseDate(w),-7)));}if(e.altKey&&e.key==='ArrowRight'){e.preventDefault();setWeek(w=>iso(addDays(parseDate(w),7)));}};window.addEventListener('keydown',onKey);return()=>window.removeEventListener('keydown',onKey);},[updatesOpen]);
  const refresh=useCallback(async()=>{applyWeek(week,await db.loadWeek(week));setError('');},[week,applyWeek]);
  const run=async(fn:()=>Promise<unknown>)=>{try{await fn();await refresh();}catch(e){setError(`Salvataggio non riuscito: ${String(e)}`);}};
  const actions:Actions={onAdd:(day,value,position)=>run(()=>db.addEntry(week,day,value,position)),onUpdate:(entry,value)=>run(()=>db.updateEntry(entry.id,value)),onToggle:entry=>run(()=>db.setCompleted(entry.id,!entry.completed)),onDelete:entry=>run(()=>db.deleteEntry(entry.id)),onDuplicate:entry=>run(()=>db.duplicateEntry(entry.id)),onMove:(entry,day)=>run(()=>db.moveEntry(entry.id,day)),onSetTime:(entry,time)=>run(()=>db.setEntryTime(entry.id,time))};
  const exportAll=async()=>{if(loadedWeek!==week)return;try{const path=await save({title:'Esporta tutti i dati del planner',defaultPath:`planner-${iso(new Date())}.json`,filters:[{name:'Dati JSON',extensions:['json']}]});if(!path)return;await notesHandle.current?.flush();await db.flushWrites();await db.exportData(path);setStatus('Dati esportati: '+path);setError('');}catch(e){setError(`Esportazione non riuscita: ${String(e)}`);}};

  const prepareImport=async()=>{if(operation.current)return;operation.current=true;setImportBusy(true);try{await notesHandle.current?.flush();await db.flushWrites();const path=await openFile({title:'Importa dati del planner',multiple:false,filters:[{name:'Dati JSON',extensions:['json']}]});if(!path){operation.current=false;return;}setImportPreview(await db.inspectImport(path));setError('');}catch(e){operation.current=false;setError(`Importazione non riuscita: ${String(e)}`);}finally{setImportBusy(false);}};
  const applyImport=async()=>{if(!importPreview||importBusy)return;setImportBusy(true);try{await notesHandle.current?.flush();await db.flushWrites();const result=await db.importData(importPreview.content);setImportPreview(null);operation.current=false;await refresh();setStatus(`Importazione completata: ${result.added} voci aggiunte, ${result.skipped} duplicati ignorati, ${result.notes_updated} note aggiornate. Backup: ${result.backup_path}`);}catch(e){setError(`Importazione non riuscita: ${String(e)}`);}finally{setImportBusy(false);if(!importPreview)operation.current=false;}};
  const beforeUpdate=async()=>{await notesHandle.current?.flush();await db.flushWrites();await db.backupData();};
  const dates=Array.from({length:7},(_,i)=>iso(addDays(parseDate(week),i)));
  return <main className="app"><MenuBar theme={theme} onTheme={setTheme} onExport={()=>void exportAll()} onImport={()=>void prepareImport()} onUpdate={()=>setUpdatesOpen(true)} exportReady={loadedWeek===week&&!importBusy&&!importPreview&&!updatesOpen} onDialog={setDialog}/><WeekNavigation week={week} onChange={next=>{if(!operation.current&&!updatesOpen)setWeek(next);}}/>{error&&<div className="error" role="alert">{error}</div>}{status&&<div className="status" role="status" onClick={()=>setStatus('')}>{status}</div>}<div className="sheet"><div className="days">{dates.slice(0,5).map((date,i)=><DayColumn key={date} date={date} index={i} entries={data.entries.filter(e=>e.day_date===date)} actions={actions}/>)}<div className="weekend">{dates.slice(5).map((date,i)=><DayColumn key={date} date={date} index={i+5} entries={data.entries.filter(e=>e.day_date===date)} actions={actions}/>)}</div></div><WeeklyNotes key={week} week={week} value={data.notes} ready={loadedWeek===week} base={notesBaseFor} handle={notesHandle} onError={setError} onChange={setNotes} onSaved={onNotesSaved}/></div><footer>Invio salva · Maiusc+Invio va a capo · Ctrl+T torna a oggi</footer>{(importBusy||importPreview)&&<div className="modal-backdrop"><section className="modal" role="dialog" aria-modal="true" aria-labelledby="import-title"><h2 id="import-title">Importa dati</h2>{importPreview?<><p>Il file contiene {importPreview.entries} voci e {importPreview.notes} settimane di note.</p><p>Le voci identiche saranno ignorate. Le altre saranno aggiunte, anche se hanno lo stesso ID. Le note saranno unite conservando il testo esistente.</p><p>Prima dell'importazione verrà creata una copia del database.</p>{error&&<p className="error" role="alert">{error}</p>}<div className="dialog-actions"><button disabled={importBusy} onClick={()=>{setImportPreview(null);operation.current=false;}}>Annulla</button><button disabled={importBusy} onClick={()=>void applyImport()}>{importBusy?'Importazione…':'Importa e unisci'}</button></div></>:<p role="status">Lettura del file…</p>}</section></div>}{updatesOpen&&<UpdateDialog version={appVersion} beforeInstall={beforeUpdate} onClose={()=>setUpdatesOpen(false)}/>} {dialog&&<InfoDialog name={dialog} version={appVersion} onClose={()=>setDialog(null)}/>}</main>;
}
