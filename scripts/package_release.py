#!/usr/bin/env python3
"""Package an existing macOS build without launching the app or any UI test."""
from pathlib import Path
import hashlib
import plistlib
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
source = root / 'build/text-md.app'
info = plistlib.loads((source / 'Contents/Info.plist').read_bytes())
version = info['CFBundleShortVersionString']
assert version == '0.3.0', 'This release script packages text-md 0.3.0.'
architectures = subprocess.check_output(['lipo', '-archs', str(source / 'Contents/MacOS/text-md')], text=True).strip()
assert architectures == 'arm64', 'Expected the Apple Silicon build.'
subprocess.run(['codesign', '--verify', '--deep', '--strict', str(source)], check=True)
output = root / 'build/releases' / version
output.mkdir(parents=True, exist_ok=True)
stem = f'text-md-{version}-macos-arm64'
dmg = output / (stem + '.dmg')
archive = output / (stem + '.zip')
assert not dmg.exists() and not archive.exists(), 'Release artifacts already exist; keep published files immutable.'
with tempfile.TemporaryDirectory(prefix='text-md-release-') as temporary:
    stage = Path(temporary)
    app = stage / 'text-md.app'
    subprocess.run(['ditto', str(source), str(app)], check=True)
    # SwiftPM's build directory can contain bundles from earlier unit-test builds.
    resources = app / 'Contents/Resources'
    for bundle in resources.glob('*Tests.bundle'):
        shutil.rmtree(bundle)
    for name in ('LICENSE', 'THIRD_PARTY_NOTICES.md'):
        shutil.copy2(root / name, resources / name)
    shutil.copy2(root / 'LICENSE', stage / 'LICENSE.txt')
    shutil.copy2(root / 'THIRD_PARTY_NOTICES.md', stage / 'THIRD_PARTY_NOTICES.md')
    shutil.copy2(root / 'Sources/TextMDCore/Resources/OPENAI-TOKENIZER-LICENSE.txt', stage / 'OPENAI-TOKENIZER-LICENSE.txt')
    (stage / 'Applications').symlink_to('/Applications', target_is_directory=True)
    (stage / 'COMO INSTALAR.txt').write_text(f'''text-md {version} — Mac Apple Silicon (M1 ou posterior)
Requer macOS {info['LSMinimumSystemVersion']} ou posterior. Não inclui versão Intel.

1. Se já usa o text-md, encerre o app normalmente antes de atualizar.
2. Arraste text-md.app para Applications / Aplicativos.
3. Ejete esta imagem e abra o app em Aplicativos.

Se o macOS bloquear por não identificar o desenvolvedor, tente abrir o app e
use Ajustes do Sistema > Privacidade e Segurança > Abrir Mesmo Assim.
Esta versão não tem certificado Developer ID nem notarização da Apple.
Orientação oficial: https://support.apple.com/en-us/102445

Seus rascunhos ficam em ~/Library/Application Support/text-md/.
Substituir o aplicativo não apaga essa biblioteca.

Código e licença MIT: https://github.com/cassianodiniz/text-md
''', encoding='utf-8')
    subprocess.run(['codesign', '--force', '--deep', '--sign', '-', str(app)], check=True)
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
    assert plistlib.loads((app / 'Contents/Info.plist').read_bytes()) == info
    assert (app / 'Contents/MacOS/text-md').stat().st_mode & 0o111
    subprocess.run(['ditto', '-c', '-k', '--sequesterRsrc', '--keepParent', str(app), str(archive)], check=True)
    subprocess.run(['hdiutil', 'create', '-quiet', '-volname', f'text-md {version}',
                    '-srcfolder', str(stage), '-fs', 'HFS+', '-format', 'UDZO', str(dmg)], check=True)
subprocess.run(['hdiutil', 'verify', str(dmg)], check=True)
checksums = output / 'SHA256SUMS.txt'
checksums.write_text(''.join(hashlib.sha256(path.read_bytes()).hexdigest() + '  ' + path.name + '\n'
                             for path in (dmg, archive)), encoding='utf-8')
for path in (dmg, archive, checksums):
    print(path)
