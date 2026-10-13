#!/usr/bin/env python3
"""Reproducible original boards, scrambled by legal reverse pulls; not run at game startup."""
import random, json
from pathlib import Path
root = Path(__file__).resolve().parent.parent
rng = random.Random(82026)
dirs = [(0,-1,'u','d'),(0,1,'d','u'),(-1,0,'l','r'),(1,0,'r','l')]
levels = [
 ('First push', ['#######','#     #','# @$. #','#     #','#######'], 'r'),
 ('Room to turn', ['#######','#  .  #','#  $  #','# @   #','#     #','#######'], 'ru'),
 ('Two deliveries', ['########','#      #','# .  . #','# $  $ #','# @    #','#      #','########'], 'udrrru'),
]
names = ['Side streets','Quiet courtyard','Crossing paths','The workshop','Night shift','Switchback','Small detour','After hours','The arcade','Inner garden','Interchange','Loading dock','The long way','Corner office','Last delivery']
for n, name in enumerate(names):
    w, h = (7 + n//6, 7 + n//10)
    count = 2 + n//6
    best = None
    for attempt in range(90):
        floor = {(x,y) for y in range(1,h-1) for x in range(1,w-1)}
        for _ in range(2+n//4): floor.discard((rng.randrange(2,w-2),rng.randrange(2,h-2)))
        goals = set(rng.sample(sorted(floor), count))
        crates = goals.copy()
        p = rng.choice(sorted(floor-crates))
        history = []
        pulls = 0
        for _ in range(350):
            dx,dy,key,inv = rng.choice(dirs)
            q = (p[0]+dx,p[1]+dy)
            behind = (p[0]-dx,p[1]-dy)
            if q not in floor or q in crates: continue
            pull = behind in crates and rng.random() < .85
            if pull: crates.remove(behind); crates.add(p); pulls += 1
            history.append(inv); p = q
        if crates == goals or pulls < 8 or len(crates-goals) != count: continue
        score = sum(min(abs(x-a)+abs(y-b) for a,b in goals) for x,y in crates)
        if best is None or score > best[0]: best = (score,floor,goals,crates.copy(),p,''.join(reversed(history)))
    assert best, name
    _,floor,goals,crates,p,solution = best
    rows=[]
    for y in range(h):
        row=''
        for x in range(w):
            q=(x,y)
            row += ('+' if q in goals else '@') if q==p else ('*' if q in goals else '$') if q in crates else '.' if q in goals else ' ' if q in floor else '#'
        rows.append(row)
    levels.append((name,rows,solution))
(root/'tests/fixtures/first-collection.xsb').write_text('\n\n'.join('; '+name+'\n'+'\n'.join(rows) for name,rows,_ in levels)+'\n')
(root/'tests/first-collection-solutions.json').write_text(json.dumps([s for _,_,s in levels],indent=2)+'\n')
print(f'Generated {len(levels)} original levels with solution witnesses')
