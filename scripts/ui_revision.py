#!/usr/bin/env python3
"""Real macOS window checks for editable diff, against an owned temporary library only.

Run after build.py. Refuses an existing text-md session and targets its child PID.
Preserves every clipboard flavor and captures only the owned app's window.
"""
from pathlib import Path
import hashlib
import json
import os
import sqlite3
import subprocess
import tempfile
import time
import uuid

root = Path(__file__).resolve().parents[1]
app = root / "build/text-md.app"
if subprocess.run(["pgrep", "-x", "text-md"], stdout=subprocess.DEVNULL).returncode == 0:
    raise SystemExit("Há uma sessão do text-md aberta. O teste não tocou nela.")
test_root = Path(tempfile.mkdtemp(prefix="text-md-revision-")).resolve()
folder = test_root / "Fixados"
folder.mkdir()
library = test_root / "library"
library.mkdir()
original = "---\nname: exemplo\ndescription: Uma skill de exemplo.\n---\n\n# Writing Skills\n\n## Overview\n\n**Texto antigo.**\n\nLeia [implement.md](references/implement.md) para continuar.\n\nÚltima linha.\n"
source = folder / "Original.md"
source.write_text(original, encoding="utf-8")
other = folder / "Outro.md"
other.write_text("# Outro arquivo\n", encoding="utf-8")
draft_id = str(uuid.uuid4()).upper()
now = time.time() - 978307200  # Foundation's JSON Date epoch.
draft = dict(id=draft_id, content=original, sourcePath=str(source), baselineHash=hashlib.sha256(original.encode()).hexdigest(),
             createdAt=now, modifiedAt=now, pinned=False, selection=0, scrollPosition=0, hasWrittenContent=True)
session = dict(tabs=[draft_id], selectedID=draft_id, folders=[dict(id=str(uuid.uuid4()).upper(), path=str(folder))],
               sourceMode=False, showLineNumbers=False, expandedPaths=[str(folder)], appearance="light",
               metrics=["lines", "words", "characters", "tokens"], ignoreMarkdown=True, tokenProfile="o200k_base", diffEnabled=False)
database = library / "library.sqlite"
with sqlite3.connect(database) as connection:
    connection.executescript("CREATE TABLE drafts(id TEXT PRIMARY KEY,json BLOB NOT NULL); CREATE TABLE settings(id TEXT PRIMARY KEY,json BLOB NOT NULL);")
    connection.execute("INSERT INTO drafts VALUES(?,?)", (draft_id, json.dumps(draft).encode()))
    connection.execute("INSERT INTO settings VALUES('session',?)", (json.dumps(session).encode(),))

helper = test_root / "native.swift"
helper.write_text('''import AppKit
import CoreGraphics
let args = CommandLine.arguments
let pasteboard = NSPasteboard.general
let url = URL(fileURLWithPath: args[2])
if args[1] == "store" {
    let items = (pasteboard.pasteboardItems ?? []).map { item in
        Dictionary(uniqueKeysWithValues: item.types.compactMap { type in item.data(forType: type).map { (type.rawValue, $0.base64EncodedString()) } })
    }
    try JSONEncoder().encode(items).write(to: url)
} else if args[1] == "restore" {
    let values = try JSONDecoder().decode([[String:String]].self, from: Data(contentsOf: url))
    let items = values.map { value in
        let item = NSPasteboardItem()
        for (type, encoded) in value { if let data = Data(base64Encoded: encoded) { item.setData(data, forType: .init(type)) } }
        return item
    }
    pasteboard.clearContents(); pasteboard.writeObjects(items)
} else {
    let pid = Int(args[2])!
    let windows = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as? [[String:Any]] ?? []
    if let window = windows.first(where: { ($0[kCGWindowOwnerPID as String] as? Int) == pid && ($0[kCGWindowLayer as String] as? Int) == 0 }),
       let id = window[kCGWindowNumber as String] { print(id) }
}
''')
native = test_root / "native-helper"
subprocess.run(["swiftc", "-module-cache-path", str(root / ".build/module-cache"), str(helper), "-o", str(native)], check=True)
clipboard = test_root / "clipboard.json"
subprocess.run([str(native), "store", str(clipboard)], check=True)
environment = dict(os.environ, TEXTMD_LIBRARY_PATH=str(library))
process = subprocess.Popen([str(app / "Contents/MacOS/text-md")], env=environment)

def quote(value):
    return '"' + str(value).replace('\\', '\\\\').replace('"', '\\"') + '"'

def applescript(body):
    helpers = '''on findID(elements, wanted)
        tell application "System Events"
            repeat with itemRef in elements
                try
                    set actualID to value of attribute "AXIdentifier" of itemRef
                    if actualID is wanted then return itemRef
                    if wanted starts with "file:/private/var/" or wanted starts with "folder:/private/var/" then
                        set savedDelimiters to AppleScript's text item delimiters
                        set AppleScript's text item delimiters to "/private/var/"
                        set parts to text items of wanted
                        set AppleScript's text item delimiters to "/var/"
                        set alternate to parts as text
                        set AppleScript's text item delimiters to savedDelimiters
                        if actualID is alternate then return itemRef
                    end if
                end try
            end repeat
        end tell
        error "Controle ausente: " & wanted
    end findID
    '''
    code = helpers + f'''tell application "System Events"
        set testingProcess to first application process whose unix id is {process.pid}
        tell testingProcess
            set frontmost to true
            try
                perform action "AXRaise" of window "text-md"
            end try
            {body}
        end tell
    end tell'''
    return subprocess.check_output(["osascript", "-e", code], text=True, stderr=subprocess.PIPE, timeout=20).strip()

def element(identifier):
    return f'my findID(entire contents of window "text-md", {quote(identifier)})'

area = element("markdown-editor")

def ready():
    last = ""
    for _ in range(60):
        try:
            if applescript(f'return exists {area}') == "true":
                applescript('set frontmost to true\n delay 0.2')
                return
        except subprocess.CalledProcessError as error:
            last = error.stderr
        time.sleep(0.1)
    raise RuntimeError("A janela não ficou pronta: " + last)

def settings():
    with sqlite3.connect(database) as connection:
        return json.loads(connection.execute("SELECT json FROM settings WHERE id='session'").fetchone()[0])

def content():
    with sqlite3.connect(database) as connection:
        return json.loads(connection.execute("SELECT json FROM drafts WHERE id=?", (draft_id,)).fetchone()[0])["content"]

def wait_for(predicate, message):
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.1)
    raise AssertionError(message)

def capture(name):
    window_id = subprocess.check_output([str(native), "window", str(process.pid)], text=True).strip()
    assert window_id, "Janela do teste não encontrada"
    subprocess.run(["screencapture", "-x", "-l", window_id, str(test_root / (name + ".png"))], check=True)

def paste(text):
    path = test_root / "paste.txt"
    path.write_text(text, encoding="utf-8")
    subprocess.run(["pbcopy"], input=text.encode(), check=True)
    applescript(f'''set frontmost to true
        delay 0.2
        set target to {area}
        set xy to position of target
        click at {{(item 1 of xy) + 50, (item 2 of xy) + 30}}
        set value of attribute "AXFocused" of target to true
        delay 0.1
        keystroke "a" using command down
        keystroke "v" using command down
        delay 0.4''')

def click_edge(identifier, edge="right"):
    applescript(f'''set target to {element(identifier)}
        set xy to position of target
        set wh to size of target
        if {quote(edge)} is "right" then
            set xx to (item 1 of xy) + (item 1 of wh) - 40
        else
            set xx to (item 1 of xy) + 4
        end if
        click at {{xx, (item 2 of xy) + (item 2 of wh) div 2}}
        delay 0.2''')

try:
    ready()
    time.sleep(0.7)
    applescript(f'click {element("toggle-search")}\n delay 0.2')
    applescript(f'set value of attribute "AXFocused" of {element("sidebar-search")} to true\n delay 0.2')
    capture("formatted-light")
    applescript(f'click {element("folder:" + str(folder))}\n delay 0.2')
    wait_for(lambda: os.path.realpath(settings().get("currentFolder", "/")) == str(folder), "Pasta não foi selecionada")
    applescript(f'click {element("file:" + str(other))}\n delay 0.2')
    assert applescript(f'return value of {area}') == "# Outro arquivo", "Arquivo não abre seu conteúdo"
    applescript(f'click {element("file:" + str(source))}\n delay 0.2')
    assert settings()["selectedID"] == draft_id and len(settings()["tabs"]) == 1, "O mesmo arquivo abriu um segundo rascunho"
    wait_for(lambda: applescript(f'return value of {area}') == original.rstrip("\n"), "Editor não selecionou o arquivo original")
    applescript(f'click {element("toggle-diff")}\n delay 0.2')
    assert settings()["diffEnabled"] is True
    current = original.replace("**Texto antigo.**", "**Texto novo.**").replace("Última linha.\n", "")
    paste(current)
    wait_for(lambda: content() == current, "Persistência contém texto incorreto")
    assert source.read_text() == original, "Editor alterou o original"
    capture("diff-light")
    applescript(f'set value of attribute "AXFocused" of {area} to true\n keystroke "a" using command down\n keystroke "c" using command down\n delay 0.1')
    assert subprocess.check_output(["pbpaste"]).decode() == current, "Command+C incluiu texto removido"
    # Check menu contents through the actual accessible menu, including no time.
    applescript(f'click {element("workspace-options")}\n delay 0.1\n set target to {element("workspace-options")}\n click menu item "Contadores" of menu 1 of target\n delay 0.1')
    menu = applescript(f'set target to {element("workspace-options")}\n return name of menu items of menu 1 of menu item "Contadores" of menu 1 of target')
    assert "Minutos" not in menu and "leitura" not in menu.lower(), "Tempo de leitura reapareceu"
    applescript(f'key code 53\n key code 53\n click {element("workspace-options")}\n delay 0.1\n set target to {element("workspace-options")}\n click menu item "Contadores" of menu 1 of target\n delay 0.1\n click menu item "Ignorar Markdown" of menu 1 of menu item "Contadores" of menu 1 of target\n delay 0.2')
    wait_for(lambda: settings()["ignoreMarkdown"] is False, "Menu de contagem não atualizou a preferência")
    assert settings()["ignoreMarkdown"] is False
    applescript(f'click {element("workspace-options")}\n delay 0.1\n set target to {element("workspace-options")}\n click menu item "Aparência" of menu 1 of target\n delay 0.1\n click menu item "Escura" of menu 1 of menu item "Aparência" of menu 1 of target\n delay 0.3')
    wait_for(lambda: settings()["appearance"] == "dark", "Menu de aparência não selecionou o tema escuro")
    capture("diff-dark")
    # Deletion-only diff must remain visible even when the native storage is empty.
    paste("")
    wait_for(lambda: content() == "", "Apagar texto no diff não persistiu")
    capture("diff-empty")
    paste(current)
    wait_for(lambda: content() == current, "Texto não ficou guardado antes de interromper")
    process.kill(); process.wait(timeout=5)
    process = subprocess.Popen([str(app / "Contents/MacOS/text-md")], env=environment)
    ready()
    assert applescript(f'return value of {area}') == current.rstrip("\n"), "Rascunho não voltou após interrupção"
    assert settings()["appearance"] == "dark" and settings()["ignoreMarkdown"] is False and settings()["diffEnabled"] is True
    assert source.read_text() == original
    print("UI: seleção da lateral, diff editável, cópia limpa, menus, tema e recuperação confirmados.")
except subprocess.CalledProcessError as error:
    print("Diagnóstico da UI:", error.stderr or str(error))
    raise
except AssertionError:
    print("Estado da janela:", applescript(f'return {{frontmost, value of attribute "AXFocused" of {area}, value of {element("sidebar-search")}}}'))
    capture("failure")
    raise
finally:
    if process.poll() is None:
        try:
            applescript('set frontmost to true\n delay 0.1\n keystroke "q" using command down')
            process.wait(timeout=5)
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
            process.terminate(); process.wait(timeout=5)
    subprocess.run([str(native), "restore", str(clipboard)], check=True)
    clipboard.unlink(missing_ok=True)
    print("Artefatos do teste:", test_root)
