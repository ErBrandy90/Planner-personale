import json
import os
import sqlite3
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SERVER = ROOT / "src-tauri" / "target" / "release" / "planner_mcp.exe"

def send(process, message):
    process.stdin.write(json.dumps(message) + "\n")
    process.stdin.flush()

def receive(process):
    line = process.stdout.readline()
    if not line:
        raise RuntimeError(process.stderr.read())
    return json.loads(line)

def call(process, request_id, name, arguments):
    send(process, {"jsonrpc": "2.0", "id": request_id, "method": "tools/call", "params": {"name": name, "arguments": arguments}})
    return receive(process)["result"]

with tempfile.TemporaryDirectory() as directory:
    database = Path(directory) / "planner.sqlite3"
    connection = sqlite3.connect(database)
    connection.executescript("""
        CREATE TABLE entries (
          id INTEGER PRIMARY KEY, week_start TEXT NOT NULL, day_date TEXT NOT NULL,
          content TEXT NOT NULL, position INTEGER NOT NULL, detected_time TEXT,
          kind TEXT NOT NULL, completed INTEGER NOT NULL DEFAULT 0, scheduled_time TEXT
        );
        CREATE TABLE weekly_notes (week_start TEXT PRIMARY KEY, content TEXT NOT NULL);
    """)
    connection.close()

    environment = os.environ.copy()
    environment["PLANNER_DB_PATH"] = str(database)
    process = subprocess.Popen([SERVER], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=environment)
    try:
        send(process, {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "planner-test", "version": "1.0"}}})
        assert "result" in receive(process)
        send(process, {"jsonrpc": "2.0", "method": "notifications/initialized"})

        created = call(process, 2, "create_task", {"title": "Test MCP", "date": "2026-10-13", "time": "10:30"})
        created_data = json.loads(created["content"][0]["text"])
        assert created_data["title"] == "Test MCP"
        assert created_data["time"] == "10:30"

        listed = call(process, 3, "list_tasks", {"date_from": "2026-10-13", "date_to": "2026-10-13", "include_completed": True})
        listed_data = json.loads(listed["content"][0]["text"])
        assert len(listed_data) == 1
        assert listed_data[0]["completed"] is False

        completed = call(process, 4, "set_task_completed", {"id": created_data["id"], "completed": True})
        completed_data = json.loads(completed["content"][0]["text"])
        assert completed_data["completed"] is True

        notes = call(process, 5, "add_weekly_notes", {"week_of": "2026-10-13", "items": ["Definire la data della riunione", "Definire la data della riunione"]})
        notes_data = json.loads(notes["content"][0]["text"])
        assert notes_data["added"] == ["Definire la data della riunione"]
        assert notes_data["already_present"] == ["Definire la data della riunione"]
    finally:
        process.terminate()
        process.wait(timeout=5)

print("MCP planner: create, list, complete e weekly notes OK")
