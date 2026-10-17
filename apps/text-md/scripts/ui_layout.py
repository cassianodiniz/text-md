#!/usr/bin/env python3
"""Check the titlebar, shared tab track and options using only an owned fixture.

Never types into a document or reads the user's library/clipboard. All native UI
queries and clicks target the exact PID launched here, even if another app is open.
"""
from pathlib import Path
import json
import os
import sqlite3
import subprocess
import tempfile
import time
import uuid

root = Path(__file__).resolve().parents[1]
test_root = Path(tempfile.mkdtemp(prefix="text-md-layout-")).resolve()
library = test_root / "library"
library.mkdir()
ids = [str(uuid.uuid4()).upper() for _ in range(2)]
texts = ["# Rascunho de postagem\n\nUma ideia para escrever com calma.\n", "# Minha skill\n\nLeia **as instruções** e continue.\n"]
now = time.time() - 978307200
session = dict(tabs=ids, selectedID=ids[0], folders=[], sourceMode=False, showLineNumbers=False,
               expandedPaths=[], appearance="light", metrics=["words", "tokens"], ignoreMarkdown=True,
               tokenProfile="o200k_base", diffEnabled=False)
database = library / "library.sqlite"
with sqlite3.connect(database) as connection:
    connection.executescript("CREATE TABLE drafts(id TEXT PRIMARY KEY,json BLOB NOT NULL); CREATE TABLE settings(id TEXT PRIMARY KEY,json BLOB NOT NULL);")
    for index, draft_id in enumerate(ids):
        draft = dict(id=draft_id, content=texts[index], createdAt=now, modifiedAt=now,
                     pinned=False, selection=0, scrollPosition=0, hasWrittenContent=True)
        connection.execute("INSERT INTO drafts VALUES(?,?)", (draft_id, json.dumps(draft).encode()))
    connection.execute("INSERT INTO settings VALUES('session',?)", (json.dumps(session).encode(),))
helper = test_root / "window.swift"
helper.write_text('''import CoreGraphics
let pid = Int(CommandLine.arguments[1])!
let windows = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as? [[String:Any]] ?? []
if let window = windows.first(where: { ($0[kCGWindowOwnerPID as String] as? Int) == pid && ($0[kCGWindowLayer as String] as? Int) == 0 }),
   let id = window[kCGWindowNumber as String] { print(id) }
''')
native = test_root / "window-helper"
subprocess.run(["swiftc", "-module-cache-path", str(root / ".build/module-cache"), str(helper), "-o", str(native)], check=True)
process = subprocess.Popen([str(root / "build/text-md.app/Contents/MacOS/text-md")],
                           env=dict(os.environ, TEXTMD_LIBRARY_PATH=str(library)))

def script(body):
    code = '''on findID(elements, wanted)
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
        tell (first application process whose unix id is {process.pid})
            set frontmost to true
            try
                perform action "AXRaise" of window "text-md"
            end try
            {body}
        end tell
    end tell'''
    return subprocess.check_output(["osascript", "-e", code], text=True, stderr=subprocess.PIPE, timeout=15).strip()

def element(identifier):
    return 'my findID(entire contents of window "text-md", ' + json.dumps(identifier) + ')'

def settings():
    with sqlite3.connect(database) as connection:
        return json.loads(connection.execute("SELECT json FROM settings WHERE id='session'").fetchone()[0])

def wait_for(predicate, message):
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.1)
    raise AssertionError(message)

def capture(name):
    window_id = subprocess.check_output([str(native), str(process.pid)], text=True).strip()
    assert window_id, "Janela do teste ausente"
    subprocess.run(["screencapture", "-x", "-l", window_id, str(test_root / (name + ".png"))], check=True)

def bounds(identifier):
    value = script(f'set target to {element(identifier)}\n return {{position of target, size of target}}')
    return [int(float(part)) for part in value.split(", ")]

def menu(submenu, item):
    script(f'''click {element("workspace-options")}
        delay 0.1
        set target to {element("workspace-options")}
        click menu item "{submenu}" of menu 1 of target
        delay 0.1
        click menu item "{item}" of menu 1 of menu item "{submenu}" of menu 1 of target
        delay 0.2''')

try:
    for _ in range(60):
        try:
            script(f'return value of {element("markdown-editor")}')
            break
        except subprocess.CalledProcessError:
            time.sleep(0.1)
    else:
        raise RuntimeError("Janela do teste não ficou pronta")
    time.sleep(0.6)
    # Move focus away from the heading to observe the formatted presentation.
    script(f'click {element("toggle-search")}\n delay 0.1\n set value of attribute "AXFocused" of {element("sidebar-search")} to true')
    first, second, counts = bounds("tab:" + ids[0]), bounds("tab:" + ids[1]), bounds("document-counters")
    editor = bounds("markdown-editor")
    assert abs(first[2] - second[2]) <= 1, "Abas não dividem a largura igualmente"
    assert first[0] < editor[0] - 100, "Faixa de abas continua limitada à coluna do editor"
    assert counts[1] < first[1], "Contadores não estão na barra superior"
    for identifier in ("toggle-diff", "toggle-markdown", "document-history", "workspace-options"):
        assert bounds(identifier)[1] < first[1], "Ação fora da barra superior: " + identifier
    capture("layout-light")
    script(f'click {element("tab:" + ids[1])}\n delay 0.2')
    assert settings()["selectedID"] == ids[1], "Selecionar aba não funciona"
    assert script(f'return value of {element("markdown-editor")}') == texts[1].rstrip("\n")
    script(f'click {element("workspace-options")}\n delay 0.1')
    names = script(f'set target to {element("workspace-options")}\n return name of menu items of menu 1 of target')
    assert all(name in names for name in ("Contadores", "Aparência", "Números de linha", "Salvar no arquivo", "Formatar"))
    capture("options")
    script(f'key code 53\n key code 53\n click {element("workspace-options")}\n delay 0.1\n set target to {element("workspace-options")}\n click menu item "Contadores" of menu 1 of target\n delay 0.1\n click menu item "Ignorar Markdown" of menu 1 of menu item "Contadores" of menu 1 of target\n delay 0.2')
    wait_for(lambda: settings()["ignoreMarkdown"] is False, "Opção Ignorar Markdown não mudou")
    menu("Aparência", "Escura")
    wait_for(lambda: settings()["appearance"] == "dark", "Tema escuro não foi selecionado")
    capture("layout-dark")
    script(f'click {element("toggle-diff")}\n delay 0.2')
    assert settings()["diffEnabled"] is True
    capture("diff-dark")
    script(f'click {element("toggle-diff")}\n click {element("toggle-markdown")}\n delay 0.2')
    assert settings()["diffEnabled"] is False and settings()["sourceMode"] is True
    script(f'click {element("document-history")}\n delay 0.2\n return exists sheet 1 of window "text-md"')
    assert script('return exists sheet 1 of window "text-md"') == "true", "Versões não abre"
    print("UI: contadores e ações na barra superior, abas iguais em toda a janela, seleção, opções, tema, Diff, Markdown e versões confirmados.")
finally:
    # Terminate only our process. Fixture content is seeded and never edited.
    if process.poll() is None:
        process.terminate()
        process.wait(timeout=5)
    print("Artefatos da verificação:", test_root)
