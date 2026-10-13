# Collegare il Planner a Claude Desktop

Il connettore usa MCP, uno standard aperto supportato da più assistenti AI. Non richiede che il Planner Settimanale sia aperto, ma l'app deve essere stata avviata almeno una volta per creare il database locale.

## Installazione in Claude Desktop

1. Apri Claude Desktop.
2. Vai in **Impostazioni → Estensioni → Impostazioni avanzate**.
3. Nella sezione per sviluppatori scegli **Installa estensione…**.
4. Seleziona il file `.dxt` del connettore (scaricalo dalle [Releases](https://github.com/ErBrandy90/Planner-personale/releases) oppure prendilo dalla cartella locale `mcp-extension`).
5. Riavvia Claude Desktop se gli strumenti non compaiono subito.

## Esempi

- “Leggi le attività del planner per i prossimi sette giorni.”
- “Controlla prima che non esista già, poi aggiungi ‘Chiamare il commercialista’ martedì 13 ottobre 2026 alle 10:30.”
- “Segna come completata l’attività ‘Chiamare il commercialista’.”
- “Aggiungi alle note di questa settimana gli impegni che non hanno ancora una data precisa.”

Per completare una voce, Claude legge prima l'elenco e usa l'ID della voce corretta. Le attività create dal connettore sono normali checkbox: possono essere spuntate anche direttamente nel planner.

## Compatibilità futura

Il server MCP è l'eseguibile `planner_mcp.exe`. Un altro client MCP locale può avviarlo via `stdio` senza dipendere da Claude. Le operazioni disponibili sono `list_tasks`, `create_task`, `set_task_completed` e `add_weekly_notes`.
