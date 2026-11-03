import re, collections

t = open('/tmp/check.txt', encoding='utf-8', errors='ignore').read()
c = collections.Counter()
for m in re.finditer(r'note: (?:function|method) defined here\n\s*-->\s*([^\n]+)\n\s*(\d+)\s*\|\s*(.*)', t):
    loc = m.group(1)
    line = m.group(3).strip()
    name = loc.replace('\\', '/').split('/')[-1]
    c[(name, line)] += 1
for (f, l), n in c.most_common(50):
    print(n, f, l[:100])
