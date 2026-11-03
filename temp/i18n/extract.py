import os, re, collections

lit = re.compile(r'"((?:[^"\\]|\\.)*)"')
u = collections.Counter()
locs = collections.defaultdict(list)
for root, _, fs in os.walk('src/ui'):
    for f in fs:
        if not f.endswith('.rs'):
            continue
        p = os.path.join(root, f)
        for i, l in enumerate(open(p, encoding='utf-8', errors='ignore'), 1):
            for m in lit.finditer(l):
                v = m.group(1)
                if len(v) < 2:
                    continue
                if v.startswith('icons/') or v.startswith('appIcons/'):
                    continue
                if re.fullmatch(r'[a-z0-9\-_]+', v):
                    continue
                if re.fullmatch(r'[0-9a-fA-F]{2,}', v):
                    continue
                if not re.search(r'[A-Za-z]', v):
                    continue
                if re.search(r'\.(svg|png|json|toml|rs)$', v):
                    continue
                u[v] += 1
                locs[v].append(f"{p}:{i}")

with open('temp/i18n/strings.txt', 'w', encoding='utf-8') as out:
    for v, c in sorted(u.items()):
        out.write(f"{c:3d}  {v}\n")
    out.write(f"=== total unique {len(u)}\n")
print("unique", len(u))
