#!/usr/bin/env python3
"""Collects Voltkit's translatable text into po/voltkit.pot and merges it into
every po/LANG.po.

    python3 po/update.py           # update the template and the catalogs
    python3 po/update.py --check   # fail if they're out of date (for CI)

Text is collected from the Rust sources (tr, trn, trf!, trnf! and N_!) and from
the command-line help, which `voltkit i18n-strings` prints. Existing
translations are kept; new text is added untranslated, and text that's gone
from the sources is dropped. Only Python's standard library is needed.
"""
import os
import re
import subprocess
import sys
from collections import OrderedDict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PO_DIR = os.path.join(ROOT, 'po')
LANGUAGES = {'es': 'Spanish', 'de': 'German', 'fr': 'French', 'zh_CN': 'Simplified Chinese'}
PLURAL_FORMS = {'fr': 'nplurals=2; plural=(n > 1);', 'zh_CN': 'nplurals=1; plural=0;'}


# ------------------------------------------------------------- Rust sources

def rust_unescape(body):
    out, i = [], 0
    while i < len(body):
        c = body[i]
        if c != '\\':
            out.append(c)
            i += 1
            continue
        n = body[i + 1]
        if n == '\n':  # line continuation: skip the newline and leading whitespace
            i += 2
            while i < len(body) and body[i] in ' \t\r\n':
                i += 1
            continue
        if n == 'u':
            end = body.index('}', i)
            out.append(chr(int(body[i + 3:end], 16)))
            i = end + 1
            continue
        if n == 'x':
            out.append(chr(int(body[i + 2:i + 4], 16)))
            i += 4
            continue
        out.append({'n': '\n', 't': '\t', 'r': '\r', '0': '\0'}.get(n, n))
        i += 2
    return ''.join(out)


def string_literals(src):
    """(start, end, value) of every string literal outside comments."""
    i, n = 0, len(src)
    while i < n:
        if src.startswith('//', i):
            j = src.find('\n', i)
            i = n if j < 0 else j
            continue
        if src.startswith('/*', i):
            depth, i = 1, i + 2
            while depth and i < n:
                if src.startswith('/*', i):
                    depth, i = depth + 1, i + 2
                elif src.startswith('*/', i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
            continue
        boundary = i == 0 or not (src[i - 1].isalnum() or src[i - 1] == '_')
        m = re.match(r'b?r(#*)"', src[i:]) if boundary else None
        if m:
            hashes = m.group(1)
            body = i + m.end()
            end = src.index('"' + hashes, body)
            yield i, end + 1 + len(hashes), src[body:end]
            i = end + 1 + len(hashes)
            continue
        c = src[i]
        if c == "'":
            m = re.match(r"'(\\u\{[0-9a-fA-F]+\}|\\x..|\\.|[^\\'])'", src[i:])
            i += m.end() if m else 1
            continue
        if c == '"':
            j = i + 1
            while src[j] != '"':
                j += 2 if src[j] == '\\' else 1
            yield i, j + 1, rust_unescape(src[i + 1:j])
            i = j + 1
            continue
        i += 1


CALL = re.compile(r'(?<![\w:])(tr|trn|trf!|trnf!|N_!)\(\s*$')
SECOND = re.compile(r'(?<![\w:])(trn|trnf!)\(\s*"(?:[^"\\]|\\.)*"\s*,\s*$', re.S)


def from_sources():
    """msgid -> {'plural': str|None, 'refs': [..], 'comments': [..]}"""
    found = OrderedDict()
    paths = []
    for base, _, files in os.walk(os.path.join(ROOT, 'src')):
        paths += [os.path.join(base, f) for f in files if f.endswith('.rs')]
    for path in sorted(paths):
        src = open(path, encoding='utf-8').read()
        tests = src.find('#[cfg(test)]')
        rel = os.path.relpath(path, ROOT)
        lits = list(string_literals(src))
        for k, (start, end, value) in enumerate(lits):
            if 0 <= tests < start:
                break
            head = src[max(0, start - 200):start]
            m = CALL.search(head)
            if not m or SECOND.search(src[max(0, start - 600):start]):
                continue
            if 'i18n.rs' in rel and value in ('', '{}'):
                continue
            plural = None
            if m.group(1) in ('trn', 'trnf!') and k + 1 < len(lits):
                nxt = lits[k + 1]
                if re.fullmatch(r'\s*,\s*', src[end:nxt[0]]):
                    plural = nxt[2]
            line = src.count('\n', 0, start) + 1
            comments = []
            # A "// TRANSLATORS:" comment just above the call goes into the catalog.
            above = src[:src.rfind('\n', 0, start)].split('\n')
            block = []
            for l in reversed(above[-6:]):
                s = l.strip()
                if s.startswith('//'):
                    block.insert(0, s.lstrip('/').strip())
                else:
                    break
            text = ' '.join(block)
            if 'TRANSLATORS:' in text:
                comments.append(text[text.index('TRANSLATORS:'):])
            entry = found.setdefault(value, {'plural': plural, 'refs': [], 'comments': []})
            entry['refs'].append(f'{rel}:{line}')
            for c in comments:
                if c not in entry['comments']:
                    entry['comments'].append(c)
            if plural and not entry['plural']:
                entry['plural'] = plural
    return found


def from_cli():
    """The command-line help, from a build of the current sources."""
    out = subprocess.run(
        ['cargo', 'run', '--quiet', '--', 'i18n-strings'],
        cwd=ROOT, check=True, capture_output=True,
    ).stdout.decode('utf-8')
    return [s for s in out.split('\0') if s]


# ------------------------------------------------------------- .po files

def po_quote(s):
    s = s.replace('\\', '\\\\').replace('"', '\\"').replace('\t', '\\t')
    lines = s.split('\n')
    if len(lines) == 1 and len(s) < 70:
        return f'"{s}"'
    parts = [l + '\\n' for l in lines[:-1]] + ([lines[-1]] if lines[-1] else [])
    return '""\n' + '\n'.join(f'"{p}"' for p in parts)


def po_unquote(chunks):
    s = ''.join(c.strip()[1:-1] for c in chunks)
    out, i = [], 0
    while i < len(s):
        if s[i] == '\\' and i + 1 < len(s):
            out.append({'n': '\n', 't': '\t', 'r': '\r', '"': '"', '\\': '\\'}.get(s[i + 1], s[i + 1]))
            i += 2
        else:
            out.append(s[i])
            i += 1
    return ''.join(out)


def read_po(path):
    """msgid -> (forms, fuzzy); the header is under ''."""
    entries = {}
    if not os.path.exists(path):
        return entries
    block = []
    for line in open(path, encoding='utf-8').read().split('\n') + ['']:
        if line.strip():
            block.append(line)
            continue
        if not block:
            continue
        fuzzy = any(l.startswith('#,') and 'fuzzy' in l for l in block)
        fields, cur = {}, None
        for l in block:
            if l.startswith('#'):
                continue
            m = re.match(r'(msgid_plural|msgid|msgstr(?:\[\d+\])?)\s+(".*")$', l)
            if m:
                cur = m.group(1)
                fields[cur] = [m.group(2)]
            elif l.startswith('"') and cur:
                fields[cur].append(l)
        block = []
        if 'msgid' not in fields:
            continue
        msgid = po_unquote(fields['msgid'])
        if 'msgstr' in fields:
            forms = [po_unquote(fields['msgstr'])]
        else:
            forms = [po_unquote(fields[k]) for k in sorted(k for k in fields if k.startswith('msgstr['))]
        entries[msgid] = (forms, fuzzy)
    return entries


def write_po(path, messages, translations, header, nplurals=2):
    out = [header.rstrip('\n'), '']
    for msgid, info in messages.items():
        for c in info['comments']:
            out.append(f'#. {c}')
        refs = info['refs']
        if refs:
            out.append('#: ' + ' '.join(refs[:4]))
        forms, fuzzy = translations.get(msgid, ([], False))
        flags = []
        if fuzzy:
            flags.append('fuzzy')
        if re.search(r'\{\d*\}', msgid):
            flags.append('rust-format')
        if flags:
            out.append('#, ' + ', '.join(flags))
        out.append('msgid ' + po_quote(msgid))
        if info['plural']:
            out.append('msgid_plural ' + po_quote(info['plural']))
            forms = (forms + [''] * nplurals)[:nplurals]
            for i, form in enumerate(forms):
                out.append(f'msgstr[{i}] ' + po_quote(form))
        else:
            out.append('msgstr ' + po_quote(forms[0] if forms else ''))
        out.append('')
    text = '\n'.join(out)
    if path:
        open(path, 'w', encoding='utf-8').write(text)
    return text


def header(language=None, name=None):
    lines = [
        f'# {name} translation of Voltkit.' if name else '# Translatable text of Voltkit.',
        '# This file is distributed under the same license as Voltkit (MIT).',
        '# Update it with po/update.py; see po/README.md.',
        'msgid ""',
        'msgstr ""',
        '"Project-Id-Version: voltkit\\n"',
        '"Report-Msgid-Bugs-To: https://github.com/arios4k/voltkit/issues\\n"',
    ]
    if language:
        lines.append(f'"Language: {language}\\n"')
    lines += [
        '"MIME-Version: 1.0\\n"',
        '"Content-Type: text/plain; charset=UTF-8\\n"',
        '"Content-Transfer-Encoding: 8bit\\n"',
        '"Plural-Forms: %s\\n"' % PLURAL_FORMS.get(language, 'nplurals=2; plural=(n != 1);'),
    ]
    return '\n'.join(lines) + '\n'


def main():
    check = '--check' in sys.argv
    messages = from_sources()
    for s in from_cli():
        messages.setdefault(s, {'plural': None, 'refs': ['src/main.rs (command-line help)'], 'comments': []})
    stale = []
    pot = write_po(None, messages, {}, header())
    pot_path = os.path.join(PO_DIR, 'voltkit.pot')
    if check:
        if not os.path.exists(pot_path) or open(pot_path, encoding='utf-8').read() != pot:
            stale.append('voltkit.pot')
    else:
        open(pot_path, 'w', encoding='utf-8').write(pot)
    for code, name in LANGUAGES.items():
        path = os.path.join(PO_DIR, f'{code}.po')
        old = read_po(path)
        old.pop('', None)
        text = write_po(None, messages, old, header(code, name), 1 if code == 'zh_CN' else 2)
        missing = [m for m in messages if not all(old.get(m, ([''], False))[0]) or old.get(m, ([], True))[1]]
        if check:
            if open(path, encoding='utf-8').read() != text:
                stale.append(f'{code}.po')
            if missing:
                print(f'{code}.po: {len(missing)} message(s) untranslated or fuzzy', file=sys.stderr)
                for m in missing[:20]:
                    print(f'  {m!r}', file=sys.stderr)
                stale.append(f'{code}.po (incomplete)')
        else:
            open(path, 'w', encoding='utf-8').write(text)
            print(f'{code}.po: {len(messages) - len(missing)} of {len(messages)} translated')
    if stale:
        print('Out of date: ' + ', '.join(stale) + '. Run python3 po/update.py and translate what is new.', file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
