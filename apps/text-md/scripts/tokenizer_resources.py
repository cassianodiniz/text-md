#!/usr/bin/env python3
"""Development only: vendor verified OpenAI ranks and generate reference fixtures.

Requires tiktoken 0.14.0. The built app has no Python or network dependency.
"""
from pathlib import Path
import base64
import hashlib
import importlib.metadata
import json
import random
import shutil
import tiktoken

root = Path(__file__).resolve().parents[1]
resources = root / "Sources/TextMDCore/Resources"
fixtures = root / "Tests/TextMDCoreTests/Fixtures"
resources.mkdir(parents=True, exist_ok=True)
fixtures.mkdir(parents=True, exist_ok=True)
assert importlib.metadata.version("tiktoken") == "0.14.0"
hashes = {
    "o200k_base": "446a9538cb6c348e3516120d7c08b09f57c36495e2acfffe59a5bf8b0cfb1a2d",
    "cl100k_base": "223921b76ee99bde995b7ff738513eef100fb51d18c93597a113bcffe865b2a7",
}
patterns = {}
for name, expected in hashes.items():
    encoding = tiktoken.get_encoding(name)
    data = b"".join(base64.b64encode(token) + b" " + str(rank).encode() + b"\n"
                    for token, rank in sorted(encoding._mergeable_ranks.items(), key=lambda item: item[1]))
    assert hashlib.sha256(data).hexdigest() == expected
    (resources / (name + ".tiktoken")).write_bytes(data)
    patterns[name] = encoding._pat_str
(resources / "patterns.json").write_text(json.dumps(patterns, ensure_ascii=False, indent=2), encoding="utf-8")
distribution = importlib.metadata.distribution("tiktoken")
license_file = next(f for f in distribution.files if str(f).endswith("licenses/LICENSE"))
shutil.copyfile(distribution.locate_file(license_file), resources / "OPENAI-TOKENIZER-LICENSE.txt")

examples = ["", "tiktoken is great!", "Olá, ação, coração e açaí.", "お誕生日おめでとう", "2 + 2 = 4",
    "---\r\nname: minha-skill\r\ndescription: Escreva em português.\r\n---\r\n# Título\r\n",
    "# Post\n\nLeia **isto** e [a referência](references/a(b).md).\n- [ ] Fazer\n",
    "<|endoftext|> <|start|> são texto literal aqui.", "😀 👋🏽 👨‍👩‍👧‍👦 é é", "\r\n\t    \n", "can't I'M we'd you're",
    "ماذا لماذا كيف", "защо", "မြန်မာ", "汉字和中文", "1234567890 3,14", "a" * 4096,
    "_" * 1000, " " * 300, "abc\rdef\r\nghi\njkl", "**[link](https://example.com)**"]
rng = random.Random(42)
alphabet = "açãoSKILLmd ABC01239-_[]()<>*/`\n\t😀 é日本語한국어العربية"
examples += ["".join(rng.choice(alphabet) for _ in range(rng.randint(1, 300))) for _ in range(120)]
examples += ["# Skill\n" + "Use ação e revisão: [texto](arquivo.md).\n" * 300]
values = [{"text": text, "counts": {name: len(tiktoken.get_encoding(name).encode_ordinary(text)) for name in hashes}}
          for text in examples]
(fixtures / "tokenizer-reference.json").write_text(json.dumps(values, ensure_ascii=False, indent=2), encoding="utf-8")
print(f"Bundled {len(hashes)} verified encodings; {len(values)} independent reference fixtures")
