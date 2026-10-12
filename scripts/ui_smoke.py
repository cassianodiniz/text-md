#!/usr/bin/env python3
"""Regression check against the real app; never uses the user's library.

Requires the macOS accessibility permission already granted to the executor.
Refuses to run if text-md is open, so it cannot hijack a user's writing session.
"""
from pathlib import Path
import json
import os
import sqlite3
import subprocess
import tempfile
import time

root = Path(__file__).resolve().parents[1]
app = root / "build/text-md.app"
if subprocess.run(["pgrep", "-x", "text-md"], stdout=subprocess.DEVNULL).returncode == 0:
    raise SystemExit("Há uma sessão do text-md aberta. O teste não tocou nela.")

test_root = Path(tempfile.mkdtemp(prefix="text-md-ui-regression-"))
environment = dict(os.environ)
environment["TEXTMD_LIBRARY_PATH"] = str(test_root / "library")
process = subprocess.Popen([str(app / "Contents/MacOS/text-md")], env=environment)

def applescript(body):
    source = '''on findID(elements, wanted)
    tell application "System Events"
        repeat with itemRef in elements
            try
                if value of attribute "AXIdentifier" of itemRef is wanted then return itemRef
            end try
        end repeat
    end tell
    error "Controle ausente: " & wanted
end findID
''' + f'''tell application "System Events"
    set testingProcess to first application process whose unix id is {process.pid}
    tell testingProcess
        set frontmost to true
        try
            perform action "AXRaise" of window "text-md"
        end try
        {body}
    end tell
end tell'''
    return subprocess.check_output(["osascript", "-e", source], text=True, stderr=subprocess.PIPE).strip()

def element(identifier):
    return 'my findID(entire contents of window "text-md", ' + json.dumps(identifier) + ')'

area = element("markdown-editor")
try:
    last_startup_error = ""
    for _ in range(50):
        try:
            if applescript('return exists window "text-md"') == "true":
                break
        except subprocess.CalledProcessError as error:
            last_startup_error = (error.stderr or str(error)).strip()
        time.sleep(0.1)
    else:
        raise RuntimeError("A janela do teste não abriu. Último erro: " + last_startup_error)

    database = test_root / "library/library.sqlite"
    with sqlite3.connect(database) as connection:
        assert connection.execute("SELECT count(*) FROM drafts").fetchone()[0] == 0, "Rascunho intocado foi persistido"

    applescript('set frontmost to true\n delay 0.2\n keystroke "t" using command down\n delay 0.2\n keystroke "w" using command down\n delay 0.2')
    with sqlite3.connect(database) as connection:
        assert connection.execute("SELECT count(*) FROM drafts").fetchone()[0] == 0, "Abrir e fechar vazio gerou nota"

    applescript(f'''set frontmost to true
        delay 0.2
        set value of attribute "AXFocused" of {area} to true
        keystroke "Texto A"
        keystroke "t" using command down
        delay 0.3
        keystroke "Texto B"
        delay 0.4''')
    with sqlite3.connect(database) as connection:
        tab_ids = {draft["content"]: draft["id"] for draft in (json.loads(row[0]) for row in connection.execute("SELECT json FROM drafts"))}
    applescript(f'click {element("tab:" + tab_ids["Texto A"])}\n delay 0.2\n keystroke "z" using command down\n delay 0.2')
    assert applescript(f"return value of {area}") == "", "Desfazer da primeira aba não funcionou"
    applescript('keystroke "z" using {command down, shift down}\n delay 0.2')
    assert applescript(f"return value of {area}") == "Texto A", "Refazer da primeira aba não funcionou"
    applescript(f'click {element("tab:" + tab_ids["Texto B"])}\n delay 0.2')
    assert applescript(f"return value of {area}") == "Texto B", "Desfazer alterou a outra aba"

    with sqlite3.connect(database) as connection:
        drafts = [json.loads(row[0]) for row in connection.execute("SELECT json FROM drafts")]
        assert {draft["content"] for draft in drafts} == {"Texto A", "Texto B"}, "Conteúdo não persistiu"

    print("UI: vazios transitórios, desfazer/refazer por aba e persistência confirmados.")
finally:
    if process.poll() is None:
        try:
            applescript('set frontmost to true\n delay 0.2\n keystroke "q" using command down')
            process.wait(timeout=5)
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
            process.terminate()
            process.wait(timeout=5)
    # Keep the tiny owned fixture for a reproducible audit, never a user library.
    print("Biblioteca de teste:", test_root)
