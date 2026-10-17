#!/usr/bin/env python3
"""Native macOS acceptance checks; owns its PID, library, documents and captures.

Does not read the user's library, type in another app or touch the clipboard.
"""
from pathlib import Path
import hashlib, json, os, sqlite3, subprocess, tempfile, time, uuid

root = Path(__file__).resolve().parents[1]
app = root / "build/text-md.app/Contents/MacOS/text-md"
fixture = Path(tempfile.mkdtemp(prefix="textmd-browser-")).resolve()
folder = fixture / "Skills"; folder.mkdir()
child = folder / "references"; child.mkdir()
(child / "ref.md").write_text("# Referência\n\nDocumento de referência.\n",encoding="utf-8")
(folder / "Outro.md").write_text("# Outro documento\n\nSegundo documento.\n",encoding="utf-8")
(folder / "imagem.png").write_bytes(bytes.fromhex("89504e470d0a1a0a"))  # Listed, never loaded as text.
original = "---\nname: implementar\ndescription: " + "Uma descrição longa sem quebra de linha. " * 12 + "\nlicense: MIT\n---\n\n# Implementar\n\nTexto de exemplo para trabalhar com calma.\n\nLeia [a referência](references/ref.md).\n"
source = folder / "SKILL.md"; source.write_text(original,encoding="utf-8")
library = fixture / "library"; library.mkdir()
database = library / "library.sqlite"
first_id = str(uuid.uuid4()).upper()
now = time.time() - 978307200
session = dict(tabs=[first_id],selectedID=first_id,folders=[dict(id=str(uuid.uuid4()).upper(),path=str(folder))],
               sourceMode=False,showLineNumbers=False,expandedPaths=[],appearance="light",metrics=["words","tokens"],
               ignoreMarkdown=True,tokenProfile="o200k_base",diffEnabled=False,currentFolder=str(folder),sidebarMode="folders")
draft = dict(id=first_id,content=original,sourcePath=str(source),baselineHash=hashlib.sha256(original.encode()).hexdigest(),
             createdAt=now,modifiedAt=now,pinned=False,selection=0,scrollPosition=0,hasWrittenContent=True)
with sqlite3.connect(database) as c:
    c.executescript("CREATE TABLE drafts(id TEXT PRIMARY KEY,json BLOB NOT NULL); CREATE TABLE settings(id TEXT PRIMARY KEY,json BLOB NOT NULL);")
    c.execute("INSERT INTO drafts VALUES(?,?)",(first_id,json.dumps(draft).encode()))
    c.execute("INSERT INTO settings VALUES('session',?)",(json.dumps(session).encode(),))
helper = fixture / "window.swift"
helper.write_text('''import AppKit
import CoreGraphics
import ApplicationServices
let pid = Int(CommandLine.arguments[1])!
if CommandLine.arguments.count == 7 && CommandLine.arguments[2] == "drag" {
    let start = CGPoint(x: Double(CommandLine.arguments[3])!, y: Double(CommandLine.arguments[4])!)
    let end = CGPoint(x: Double(CommandLine.arguments[5])!, y: Double(CommandLine.arguments[6])!)
    func post(_ type: CGEventType, _ point: CGPoint) {
        let event = CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: point, mouseButton: .left)!
        event.flags = []; event.post(tap: .cghidEventTap)
        usleep(80000)
    }
    post(.mouseMoved,start); post(.leftMouseDown,start)
    for step in 1...16 {
        let part = CGFloat(step) / 16
        post(.leftMouseDragged,CGPoint(x:start.x+(end.x-start.x)*part,y:start.y+(end.y-start.y)*part))
    }
    post(.leftMouseUp,end); exit(0)
}
if (CommandLine.arguments.count == 4 && CommandLine.arguments[2] == "tab") || (CommandLine.arguments.count == 5 && CommandLine.arguments[2] == "bounds") {
    func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
        var value: CFTypeRef?; AXUIElementCopyAttributeValue(element,name as CFString,&value); return value
    }
    func find(_ element: AXUIElement) -> String? {
        if let id = attribute(element,"AXIdentifier") as? String, id.hasPrefix("tab:") { return id }
        for child in attribute(element,"AXChildren") as? [AXUIElement] ?? [] { if let found = find(child) { return found } }
        return nil
    }
    let windows = attribute(AXUIElementCreateApplication(pid_t(pid)),"AXWindows") as? [AXUIElement] ?? []
    guard let window = windows.first(where: { attribute($0,"AXIdentifier") as? String == CommandLine.arguments[3] }) else { exit(1) }
    if CommandLine.arguments[2] == "bounds" {
        func locate(_ element: AXUIElement) -> AXUIElement? {
            if attribute(element,"AXIdentifier") as? String == CommandLine.arguments[4] { return element }
            for child in attribute(element,"AXChildren") as? [AXUIElement] ?? [] { if let found = locate(child) { return found } }
            return nil
        }
        guard let element = locate(window), let origin = attribute(element,"AXPosition"), let size = attribute(element,"AXSize") else { exit(1) }
        var point = CGPoint.zero; var dimensions = CGSize.zero
        AXValueGetValue(origin as! AXValue,.cgPoint,&point); AXValueGetValue(size as! AXValue,.cgSize,&dimensions)
        print("\(point.x), \(point.y), \(dimensions.width), \(dimensions.height)"); exit(0)
    }
    if let id = find(window) { print(id); exit(0) }
    exit(1)
}
if CommandLine.arguments.count == 6 && CommandLine.arguments[2] == "range" {
    func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
        var value: CFTypeRef?; AXUIElementCopyAttributeValue(element,name as CFString,&value); return value
    }
    func find(_ element: AXUIElement, _ wanted: String) -> AXUIElement? {
        if attribute(element,"AXIdentifier") as? String == wanted { return element }
        for child in attribute(element,"AXChildren") as? [AXUIElement] ?? [] { if let found = find(child,wanted) { return found } }
        return nil
    }
    let application = AXUIElementCreateApplication(pid_t(pid))
    let windows = attribute(application,"AXWindows") as? [AXUIElement] ?? []
    guard let window = windows.first(where: { attribute($0,"AXIdentifier") as? String == CommandLine.arguments[3] }),
          let editor = find(window,"markdown-editor") else { exit(1) }
    var range = CFRange(location:Int(CommandLine.arguments[4])!,length:Int(CommandLine.arguments[5])!)
    let input = AXValueCreate(.cfRange,&range)!
    var value: CFTypeRef?
    guard AXUIElementCopyParameterizedAttributeValue(editor,"AXBoundsForRange" as CFString,input,&value) == .success, let value else { exit(1) }
    var rect = CGRect.zero; AXValueGetValue(value as! AXValue,.cgRect,&rect)
    print("\(rect.minX), \(rect.minY), \(rect.width), \(rect.height)"); exit(0)
}
if CommandLine.arguments.count == 5 {
    let point = CGPoint(x: Double(CommandLine.arguments[2])!, y: Double(CommandLine.arguments[3])!)
    let flags: CGEventFlags = CommandLine.arguments[4] == "command" ? .maskCommand : []
    if flags.contains(.maskCommand) {
        let key = CGEvent(keyboardEventSource:nil,virtualKey:55,keyDown:true)!
        key.flags = .maskCommand; key.post(tap:.cghidEventTap); usleep(100000)
    }
    for type in [CGEventType.mouseMoved, .leftMouseDown, .leftMouseUp] {
        let event = CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: point, mouseButton: .left)!
        event.flags = flags; event.post(tap: .cghidEventTap); usleep(100000)
    }
    if flags.contains(.maskCommand) {
        usleep(300000)
        let key = CGEvent(keyboardEventSource:nil,virtualKey:55,keyDown:false)!
        key.flags = []; key.post(tap:.cghidEventTap)
    }
    exit(0)
}
let windows = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as? [[String:Any]] ?? []
for window in windows where (window[kCGWindowOwnerPID as String] as? Int) == pid && (window[kCGWindowLayer as String] as? Int) == 0 {
    if let id = window[kCGWindowNumber as String] { print(id) }
}
''')
native=fixture/"window-helper"
subprocess.run(["swiftc","-module-cache-path",str(root/".build/module-cache"),str(helper),"-o",str(native)],check=True)
env=dict(os.environ,TEXTMD_LIBRARY_PATH=str(library))
process=subprocess.Popen([str(app)],env=env)

def quote(value): return json.dumps(str(value),ensure_ascii=False)

def script(body, tab=None, window_id=None):
    finder='''on findID(elements, wanted)
        tell application "System Events"
            repeat with itemRef in elements
                try
                    set actualID to value of attribute "AXIdentifier" of itemRef
                    if actualID is wanted then return itemRef
                    if wanted starts with "file:/private/var/" or wanted starts with "folder:/private/var/" or wanted starts with "pin-folder:/private/var/" then
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
    selection='set targetWindow to window 1'
    if tab or window_id:
        owner = window_id or next(win["id"] for win in settings()["windows"] if tab in win["tabs"])
        selection=f'''repeat with candidate in windows
            try
                if value of attribute "AXIdentifier" of candidate is not {quote(owner)} then error "Outra janela"
                set targetWindow to candidate
                exit repeat
            end try
        end repeat'''
    code=finder+f'''tell application "System Events"
        tell (first application process whose unix id is {process.pid})
            set frontmost to true
            {selection}
            perform action "AXRaise" of targetWindow
            delay 0.1
            {selection}
            set xy to position of targetWindow
            click at {{(item 1 of xy) + 220, (item 2 of xy) + 18}}
            delay 0.1
            {selection}
            {body}
        end tell
    end tell'''
    return subprocess.check_output(["osascript","-e",code],text=True,stderr=subprocess.PIPE,timeout=20).strip()

def element(identifier): return f'my findID(entire contents of targetWindow, {quote(identifier)})'

def settings():
    with sqlite3.connect(database) as c: return json.loads(c.execute("SELECT json FROM settings WHERE id='session'").fetchone()[0])

def drafts():
    with sqlite3.connect(database) as c: return [json.loads(r[0]) for r in c.execute("SELECT json FROM drafts")]

def wait_for(check,message):
    until=time.monotonic()+6
    while time.monotonic()<until:
        if check(): return
        time.sleep(0.1)
    raise AssertionError(message)

def key(value, modifiers="command down", tab=None):
    script(f'keystroke {quote(value)} using {{{modifiers}}}\n delay 0.25',tab)

def click(identifier, tab=None, command=False):
    if command:
        values=script(f'set target to {element(identifier)}\n return {{position of target,size of target}}',tab)
        x,y,w,h=[int(float(v)) for v in values.split(", ")]
        subprocess.run([str(native),str(process.pid),str(x+40),str(y+h//2),"command"],check=True)
        time.sleep(0.25)
    else: script(f'click {element(identifier)}\n delay 0.2',tab)

def bounds(identifier,tab=None,window_id=None):
    script('return position of targetWindow',tab,window_id)
    owner=window_id or next(win["id"] for win in settings()["windows"] if tab in win["tabs"]) if (window_id or tab) else settings()["windows"][0]["id"]
    values=subprocess.check_output([str(native),str(process.pid),"bounds",owner,identifier],text=True).strip()
    return [float(v) for v in values.split(", ")]

def click_point(x,y):
    subprocess.run([str(native),str(process.pid),str(x),str(y),"plain"],check=True)
    time.sleep(0.15)

def drag(start,end):
    subprocess.run([str(native),str(process.pid),"drag",*(str(v) for v in (*start,*end))],check=True)
    time.sleep(0.3)

def range_bounds(tab,position,length):
    owner=next(win["id"] for win in settings()["windows"] if tab in win["tabs"])
    result=subprocess.check_output([str(native),str(process.pid),"range",owner,str(position),str(length)],text=True)
    return [float(v) for v in result.strip().split(", ")]

def capture(name):
    ids=subprocess.check_output([str(native),str(process.pid)],text=True).splitlines()
    for index,window_id in enumerate(ids):
        subprocess.run(["screencapture","-x","-l",window_id,str(fixture/f"{name}-{index}.png")],check=True)

def menu(submenu,item):
    script(f'''key code 53
        key code 53
        click {element("workspace-options")}
        delay 0.2
        set target to {element("workspace-options")}
        click menu item {quote(submenu)} of menu 1 of target
        delay 0.2
        click menu item {quote(item)} of menu 1 of menu item {quote(submenu)} of menu 1 of target
        delay 0.2''')

try:
    for _ in range(60):
        try:
            script(f'return value of {element("markdown-editor")}'); break
        except subprocess.CalledProcessError: time.sleep(0.1)
    else: raise RuntimeError("Editor não abriu")
    time.sleep(0.4)
    capture("whitey-light")
    print("READY",flush=True)
    assert script(f'return exists {element("file:"+str(folder/"imagem.png"))}')=="true"
    assert script(f'return exists {element("metadata-scrollbar")}')=="true"
    # Horizontal metadata scrolling leaves the heading in the same position.
    heading_position=len(original[:original.index("# Implementar")].encode("utf-16-le"))//2
    before_heading=range_bounds(first_id,heading_position,1)
    x,y,w,h=bounds("metadata-scrollbar")
    click_point(x+w*0.7,y+h/2)
    wait_for(lambda: float(script(f'return value of {element("metadata-scrollbar")}'))>0,"Rolagem horizontal não percorreu as propriedades")
    after_heading=range_bounds(first_id,heading_position,1)
    assert before_heading==after_heading,"Rolagem das propriedades deslocou o título"
    capture("metadata-scrolled")
    # A normal link click opens the local Markdown reference, keeping the source.
    label_position=len(original[:original.index("a referência")].encode("utf-16-le"))//2
    x,y,w,h=range_bounds(first_id,label_position,len("a referência"))
    click_point(x+w/2,y+h/2)
    wait_for(lambda: len(settings()["tabs"])==2 and settings()["selectedID"]!=first_id,"Link Markdown não abriu a referência")
    key("w")
    wait_for(lambda: settings()["tabs"]==[first_id],"Fechar referência alterou a aba de origem")
    # Collapsing a section doesn't remove its actual pins.
    click("section:pinned")
    assert "pinned" in settings()["windows"][0]["collapsedSections"]
    path_before=settings()["currentFolder"]
    click("pin-folder:"+str(folder))
    assert settings()["folders"]==[] and settings()["currentFolder"]==path_before
    click("pin-folder:"+str(folder))
    assert len(settings()["folders"])==1 and settings()["currentFolder"]==path_before
    click("section:pinned")
    # Enter a folder, open a file in the current tab, then return to the parent.
    click("folder:"+str(child))
    wait_for(lambda: os.path.realpath(settings()["currentFolder"])==str(child),"Entrar na pasta não atualizou o caminho")
    click("file:"+str(child/"ref.md"))
    wait_for(lambda: len(settings()["tabs"])==1 and settings()["selectedID"]!=first_id,"Clique simples não substituiu a aba")
    ref_id=settings()["selectedID"]
    click("folder-up")
    click("file:"+str(source),command=True)
    wait_for(lambda: len(settings()["tabs"])==2 and settings()["selectedID"]==first_id,"Command+clique não abriu aba")
    # Global zoom across multiple windows, without writing the original.
    key("n")
    wait_for(lambda: len(settings().get("windows",[]))==2,"Command+N não criou janela")
    key("=")
    wait_for(lambda: abs(settings().get("zoom",1)-1.1)<0.001,"Command+mais não mudou o zoom")
    key("0",tab=first_id)
    wait_for(lambda: settings()["zoom"]==1,"Zoom da outra janela não foi compartilhado")
    # Opening a file already in another window goes there and creates no tab copy.
    empty_window=next(win["id"] for win in settings()["windows"] if not win["tabs"])
    script('return position of targetWindow',window_id=empty_window)
    second=subprocess.Popen([str(app),str(source)],env=env)
    assert second.wait(timeout=8)==0,"Outra instância ficou rodando"
    assert sum(first_id in win["tabs"] for win in settings()["windows"])==1
    owner=next(win["id"] for win in settings()["windows"] if first_id in win["tabs"])
    wait_for(lambda: script('return value of attribute "AXIdentifier" of window 1')==owner,"Abrir arquivo já aberto não trouxe sua janela")
    assert len(settings()["windows"])==2,"Reabrir arquivo criou janela vazia extra"
    key("t")
    script(f'set value of attribute "AXFocused" of {element("markdown-editor")} to true\n keystroke "Rascunho persistente"\n delay 0.3')
    wait_for(lambda: any(d["content"]=="Rascunho persistente" for d in drafts()),"Novo rascunho não persistiu")
    note_id=next(d["id"] for d in drafts() if d["content"]=="Rascunho persistente")
    key("w")
    assert all(note_id not in win["tabs"] for win in settings()["windows"])
    key("t","command down, shift down")
    wait_for(lambda: any(note_id in win["tabs"] for win in settings()["windows"]),"Command+Shift+T não reabriu a aba")
    # Real drag between windows, then out to a new window. Original documents
    # stay in place and the moved note keeps its ID/content throughout.
    target_id=empty_window
    script('set position of targetWindow to {20, 120}\n set size of targetWindow to {850, 650}',tab=note_id)
    script('set position of targetWindow to {400, 400}\n set size of targetWindow to {850, 650}',window_id=target_id)
    target_tab=subprocess.check_output([str(native),str(process.pid),"tab",target_id],text=True).strip()
    tx,ty,tw,th=bounds(target_tab,window_id=target_id)
    sx,sy,sw,sh=bounds("tab:"+note_id,tab=note_id)
    drag((sx+sw/2,sy+sh/2),(tx+tw*0.8,ty+th/2))
    wait_for(lambda: next(win["id"] for win in settings()["windows"] if note_id in win["tabs"])==target_id,"Arraste não transferiu a aba entre janelas")
    capture("tab-moved")
    sx,sy,sw,sh=bounds("tab:"+note_id,tab=note_id)
    drag((sx+sw/2,sy+sh/2),(1300,90))
    wait_for(lambda: len(settings()["windows"])==3,"Soltar uma aba fora não criou janela")
    assert next(d["content"] for d in drafts() if d["id"]==note_id)=="Rascunho persistente"
    # Close the untouched empty window, then the note's last tab.
    focus=script('return value of attribute "AXIdentifier" of (value of attribute "AXFocusedWindow")',window_id=target_id)
    assert focus==target_id,"Teste não conseguiu dar foco à janela vazia"
    script('keystroke "w" using command down\n delay 0.2',window_id=target_id)
    wait_for(lambda: len(settings()["windows"])==2,"Fechar última aba vazia não fechou a janela")
    key("w",tab=note_id)
    wait_for(lambda: len(settings()["windows"])==1,"Fechar última aba não fechou a janela")
    key("t","command down, shift down")
    wait_for(lambda: any(note_id in win["tabs"] for win in settings()["windows"]),"Reabrir rascunho de janela fechada falhou")
    # Menus use the native submenu interaction in one uninterrupted action.
    menu("Contadores","Ignorar Markdown")
    wait_for(lambda: settings()["ignoreMarkdown"] is False,"Ignorar Markdown não foi aplicado")
    menu("Aparência","Escura")
    wait_for(lambda: settings()["appearance"]=="dark","Tema escuro não foi aplicado")
    capture("dark-windows")
    assert source.read_text(encoding="utf-8")==original
    # Normal quit and restore multiple windows and all durable tabs.
    key("n")
    wait_for(lambda: len(settings()["windows"])==2,"Nova janela para restauração ausente")
    expected=[w["tabs"] for w in settings()["windows"]]
    key("q"); process.wait(timeout=8)
    process=subprocess.Popen([str(app)],env=env)
    wait_for(lambda: [w["tabs"] for w in settings()["windows"]]==expected,"Layout persistido foi alterado")
    for _ in range(50):
        try:
            if int(script('return count of windows'))==len(expected): break
        except subprocess.CalledProcessError: pass
        time.sleep(0.1)
    else: raise AssertionError("Janelas não foram restauradas")
    assert source.read_text(encoding="utf-8")==original
    print("UI: propriedades sem quebra e rolagem isolada, links locais, pins, navegação, clique simples/Command, arraste entre/fora de janelas, instância única, zoom global, fechar/reabrir, menus e restauração confirmados.",flush=True)
except AssertionError:
    capture("assertion-failure")
    try: print("Metadata control:",script(f'return {{enabled of {element("metadata-scrollbar")},value of {element("metadata-scrollbar")}}}'),flush=True)
    except subprocess.CalledProcessError: pass
    raise
except subprocess.CalledProcessError as error:
    print("ERRO UI:",error.stderr,flush=True)
    capture("failure")
    print(script('''set output to ""
        repeat with itemRef in entire contents of targetWindow
            try
                set output to output & (value of attribute "AXRole" of itemRef) & " | " & (value of attribute "AXIdentifier" of itemRef) & linefeed
            end try
        end repeat
        return output'''),flush=True)
    raise
finally:
    if process.poll() is None:
        try: key("q"); process.wait(timeout=5)
        except (subprocess.CalledProcessError,subprocess.TimeoutExpired): process.terminate(); process.wait(timeout=5)
    print("Artefatos:",fixture,flush=True)
