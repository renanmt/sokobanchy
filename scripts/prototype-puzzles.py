#!/usr/bin/env python3
"""Generate independent candidate puzzles, verify solvability, and measure minimum pushes.
This deliberately leaves the game's curated pack unchanged. No reference boards are read.
"""
from pathlib import Path
import itertools
import json
import random
import runpy
import time

ROOT=Path(__file__).resolve().parent.parent
analysis=runpy.run_path(str(ROOT/'scripts/analyze-levels.py'))
solve=analysis['solve_pushes']; replay=analysis['replay']; add=analysis['add']; reachable=analysis['reachable']
DIRECTIONS=analysis['DIRECTIONS']
rng=random.Random(20261013)
ranked=[]; seen=set(); started=time.monotonic(); tried=0
for attempt in range(3000):
    w,h=8,8
    floor={(x,y) for y in range(1,h-1) for x in range(1,w-1)}
    for _ in range(rng.randrange(3,8)):
        floor.discard((rng.randrange(1,w-1),rng.randrange(1,h-1)))
    if len(reachable(min(floor),set(),floor))!=len(floor): continue
    goals=set(rng.sample(sorted(floor),3)); boxes=goals.copy(); p=rng.choice(sorted(floor-boxes))
    inverse=[]; pulls=0
    for _ in range(650):
        key,(dx,dy)=rng.choice(list(DIRECTIONS.items()))
        q=add(p,(dx,dy)); behind=add(p,(-dx,-dy))
        if q not in floor or q in boxes: continue
        if behind in boxes and rng.random()<.9:
            boxes.remove(behind); boxes.add(p); pulls+=1
        p=q; inverse.append({'u':'d','d':'u','l':'r','r':'l'}[key])
    if len(boxes-goals)<2 or pulls<8: continue
    fingerprint=(tuple(sorted(floor)),tuple(sorted(goals)),tuple(sorted(boxes)),p)
    if fingerprint in seen: continue
    seen.add(fingerprint)
    level={'floor':floor,'goals':goals,'boxes':boxes,'player':p}
    witness=''.join(reversed(inverse))
    assert replay(level,witness), 'Reverse-generation witness failed'
    minimum=solve(level,max_states=16000,seconds=.6)
    tried+=1
    if minimum['status']!='optimal' or minimum['pushes']<16: continue
    lower=min(sum(abs(a[0]-b[0])+abs(a[1]-b[1]) for a,b in zip(sorted(boxes),perm)) for perm in itertools.permutations(goals))
    extra=minimum['pushes']-lower
    wall_bound=analysis['independent_push_bound'](level)
    rows=[]
    for y in range(h):
        row=''
        for x in range(w):
            q=(x,y)
            row+=('+' if q in goals else '@') if q==p else ('*' if q in goals else '$') if q in boxes else '.' if q in goals else ' ' if q in floor else '#'
        rows.append(row)
    ranked.append({'rows':rows,'witness':witness,'optimal_pushes':minimum['pushes'],'assignment_distance_lower_bound':lower,'extra_pushes_over_distance':extra,'independent_crate_push_lower_bound':wall_bound,'extra_pushes_over_independent_bound':minimum['pushes']-wall_bound,'solver_states':minimum['states']})
    print('Candidate',attempt,'minimum pushes',minimum['pushes'],'extra',extra,'states',minimum['states'],flush=True)
    if time.monotonic()-started>50: break
ranked.sort(key=lambda l:(l['extra_pushes_over_independent_bound'],l['optimal_pushes']),reverse=True)
winners=ranked[:3]
assert winners, 'No suitable candidates in this run'
for n,level in enumerate(winners): level['name']=['Switchyard','Tight quarters','Round trip'][n]
(ROOT/'analysis/original-prototypes.xsb').write_text('\n\n'.join('; '+l['name']+'\n'+'\n'.join(l['rows']) for l in winners)+'\n')
(ROOT/'analysis/original-prototypes.json').write_text(json.dumps({'seed':20261013,'candidates_solved':tried,'levels':winners},indent=2)+'\n')
print('Selected',[(l['name'],l['optimal_pushes'],l['extra_pushes_over_distance']) for l in winners],flush=True)
